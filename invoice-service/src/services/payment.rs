use std::time::Duration;

use actix_web::web;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::Row;
use uuid::Uuid;

use crate::errors::AppError;
use crate::models::invoice::Invoice;
use crate::models::payment::{PayInvoiceRequest, PaymentPendingResponse, PaymentSuccessResponse};
use crate::services::webhook::enqueue_webhook_event;
use crate::AppState;

#[derive(Debug, Serialize)]
pub struct PaymentFailureDetails {
    pub failure_code: String,
    pub payment_attempt_id: Uuid,
}

pub enum PaymentResult {
    Succeeded(PaymentSuccessResponse),
    Pending(PaymentPendingResponse),
    Failed(PaymentFailureDetails),
}

#[derive(Debug, Deserialize)]
struct MockChargeResponse {
    status: String,
    psp_ref: Option<String>,
    code: Option<String>,
}

#[derive(Debug, Serialize)]
struct MockChargeRequest {
    card_token: String,
    amount_cents: i64,
    currency: String,
    reference: String,
}

pub async fn pay_invoice(
    state: web::Data<AppState>,
    business_id: Uuid,
    invoice_id: Uuid,
    idempotency_key: String,
    payload: PayInvoiceRequest,
) -> Result<PaymentResult, AppError> {
    let request_hash = hash_json(&serde_json::json!({ "card_token": payload.card_token }));

    if let Some(existing) = sqlx::query(
        r#"
        SELECT id, request_hash, status::text AS status, psp_reference, failure_code, amount_cents, invoice_id
        FROM payment_attempts
        WHERE idempotency_key = $1
        "#,
    )
    .bind(&idempotency_key)
    .fetch_optional(&state.pool)
    .await?
    {
        let existing_request_hash: String = existing.get("request_hash");
        if existing_request_hash != request_hash {
            return Err(AppError::Conflict(
                "This idempotency key was used with a different request body".to_string(),
            ));
        }

        let existing_invoice_id: Uuid = existing.get("invoice_id");
        if existing_invoice_id != invoice_id {
            return Err(AppError::Conflict(
                "This idempotency key was used with a different invoice".to_string(),
            ));
        }

        let existing_id: Uuid = existing.get("id");
        let existing_status: String = existing.get("status");
        let existing_psp_reference: Option<String> = existing.get("psp_reference");
        let existing_failure_code: Option<String> = existing.get("failure_code");
        let existing_amount: i64 = existing.get("amount_cents");

        return Ok(match existing_status.as_str() {
            "succeeded" => PaymentResult::Succeeded(PaymentSuccessResponse {
                payment_attempt_id: existing_id,
                invoice_id: invoice_id,
                status: "succeeded".to_string(),
                psp_reference: existing_psp_reference,
                amount_cents: existing_amount,
            }),
            "failed" => PaymentResult::Failed(PaymentFailureDetails {
                failure_code: existing_failure_code.unwrap_or_else(|| "payment_failed".to_string()),
                payment_attempt_id: existing_id,
            }),
            _ => PaymentResult::Pending(PaymentPendingResponse {
                payment_attempt_id: existing_id,
                invoice_id: invoice_id,
                status: "pending".to_string(),
                message: "Payment is being processed. Check payment status by polling GET /invoices/:id"
                    .to_string(),
            }),
        });
    }

    let mut tx = state.pool.begin().await?;

    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        SELECT id, business_id, customer_id, state::text AS state, total_cents,
               due_date, description, created_at, updated_at
        FROM invoices
        WHERE id = $1 AND business_id = $2
        FOR UPDATE
        "#,
    )
    .bind(invoice_id)
    .bind(business_id)
    .fetch_optional(&mut *tx)
    .await?
    .ok_or_else(|| AppError::NotFound("Invoice not found".to_string()))?;

    if invoice.state != "open" {
        return Err(AppError::Conflict(format!(
            "Cannot pay invoice in state '{}'",
            invoice.state
        )));
    }

    let attempt = sqlx::query(
        r#"
        INSERT INTO payment_attempts (invoice_id, idempotency_key, request_hash, status, card_token, amount_cents)
        VALUES ($1, $2, $3, 'pending', $4, $5)
        RETURNING id
        "#,
    )
    .bind(invoice.id)
    .bind(idempotency_key)
    .bind(request_hash)
    .bind(payload.card_token.clone())
    .bind(invoice.total_cents)
    .fetch_one(&mut *tx)
    .await?;

    let attempt_id: Uuid = attempt.get("id");

    // Optimistically mark the invoice as paid *inside* the same transaction
    // that holds the FOR UPDATE lock.  Once this commits, every concurrent
    // request that was waiting on the lock will re-read state = 'paid' and
    // immediately return 409 — preventing any double-charge.
    // If the PSP later declines we revert the invoice back to 'open'.
    sqlx::query(
        r#"
        UPDATE invoices
        SET state = 'paid', updated_at = NOW()
        WHERE id = $1
        "#,
    )
    .bind(invoice.id)
    .execute(&mut *tx)
    .await?;

    tx.commit().await?;
    // Lock released here; concurrent readers now see state = 'paid'.

    let charge_req = MockChargeRequest {
        card_token: payload.card_token,
        amount_cents: invoice.total_cents,
        currency: "USD".to_string(),
        reference: attempt_id.to_string(),
    };

    let psp_call = state
        .http_client
        .post(format!("{}/charge", state.config.mock_psp_url))
        .json(&charge_req)
        .send();

    let timed = tokio::time::timeout(Duration::from_secs(state.config.psp_timeout_secs), psp_call).await;

    match timed {
        Ok(Ok(resp)) if resp.status().is_success() => {
            let body: MockChargeResponse = resp.json().await?;
            if body.status == "succeeded" {
                sqlx::query(
                    r#"
                    UPDATE payment_attempts
                    SET status = 'succeeded', psp_reference = $1, response_json = $2, updated_at = NOW()
                    WHERE id = $3
                    "#,
                )
                .bind(body.psp_ref.clone())
                .bind(serde_json::json!({
                        "payment_attempt_id": attempt_id,
                        "invoice_id": invoice.id,
                        "status": "succeeded",
                        "psp_reference": body.psp_ref.clone(),
                        "amount_cents": invoice.total_cents
                    }))
                .bind(attempt_id)
                .execute(&state.pool)
                .await?;

                enqueue_webhook_event(
                    state.clone(),
                    business_id,
                    "invoice.paid",
                    serde_json::json!({
                        "event": "invoice.paid",
                        "invoice_id": invoice.id,
                        "payment_attempt_id": attempt_id,
                        "amount_cents": invoice.total_cents,
                        "psp_reference": body.psp_ref.clone(),
                    }),
                )
                .await;

                Ok(PaymentResult::Succeeded(PaymentSuccessResponse {
                    payment_attempt_id: attempt_id,
                    invoice_id: invoice.id,
                    status: "succeeded".to_string(),
                    psp_reference: body.psp_ref,
                    amount_cents: invoice.total_cents,
                }))
            } else {
                // PSP declined — revert invoice to 'open' so the client can retry.
                let failure_code = body.code.unwrap_or_else(|| "payment_failed".to_string());
                sqlx::query(
                    r#"
                    UPDATE invoices
                    SET state = 'open', updated_at = NOW()
                    WHERE id = $1
                    "#,
                )
                .bind(invoice.id)
                .execute(&state.pool)
                .await?;

                sqlx::query(
                    r#"
                    UPDATE payment_attempts
                    SET status = 'failed', failure_code = $1, updated_at = NOW()
                    WHERE id = $2
                    "#,
                )
                .bind(failure_code.clone())
                .bind(attempt_id)
                .execute(&state.pool)
                .await?;

                enqueue_webhook_event(
                    state.clone(),
                    business_id,
                    "invoice.payment_failed",
                    serde_json::json!({
                        "event": "invoice.payment_failed",
                        "invoice_id": invoice.id,
                        "payment_attempt_id": attempt_id,
                        "failure_code": failure_code.clone(),
                    }),
                )
                .await;

                Ok(PaymentResult::Failed(PaymentFailureDetails {
                    failure_code,
                    payment_attempt_id: attempt_id,
                }))
            }
        }
        Ok(Ok(resp)) => {
            let error = format!("psp http error: {}", resp.status());
            sqlx::query(
                r#"
                UPDATE payment_attempts
                SET status = 'pending', failure_code = $1, updated_at = NOW()
                WHERE id = $2
                "#,
            )
            .bind(error)
            .bind(attempt_id)
            .execute(&state.pool)
            .await?;

            Ok(PaymentResult::Pending(PaymentPendingResponse {
                payment_attempt_id: attempt_id,
                invoice_id: invoice.id,
                status: "pending".to_string(),
                message: "Payment is being processed. Check payment status by polling GET /invoices/:id"
                    .to_string(),
            }))
        }
        _ => {
            sqlx::query(
                r#"
                UPDATE payment_attempts
                SET status = 'pending', failure_code = 'timeout_or_network_error', updated_at = NOW()
                WHERE id = $1
                "#,
            )
            .bind(attempt_id)
            .execute(&state.pool)
            .await?;

            Ok(PaymentResult::Pending(PaymentPendingResponse {
                payment_attempt_id: attempt_id,
                invoice_id: invoice.id,
                status: "pending".to_string(),
                message: "Payment is being processed. Check payment status by polling GET /invoices/:id"
                    .to_string(),
            }))
        }
    }
}

fn hash_json(value: &serde_json::Value) -> String {
    let bytes = serde_json::to_vec(value).unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}
