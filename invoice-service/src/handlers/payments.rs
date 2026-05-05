use actix_web::{http::StatusCode, web, HttpRequest, HttpResponse};
use uuid::Uuid;

use crate::auth::AuthenticatedBusiness;
use crate::errors::AppError;
use crate::models::payment::{PayInvoiceHeaders, PayInvoiceRequest};
use crate::response::{ApiErrorResponse, ApiResponse};
use crate::services::payment::{pay_invoice, PaymentResult};
use crate::validation::validate_request;
use crate::AppState;

pub async fn pay_invoice_handler(
    state: web::Data<AppState>,
    auth: AuthenticatedBusiness,
    path: web::Path<Uuid>,
    req: HttpRequest,
    payload: web::Json<PayInvoiceRequest>,
) -> Result<HttpResponse, AppError> {
    let headers = PayInvoiceHeaders {
        idempotency_key: req
            .headers()
            .get("Idempotency-Key")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::BadRequest("Idempotency-Key header is required".to_string()))?,
    }
    .normalize();
    validate_request(&headers)?;

    let payload = payload.into_inner().normalize();
    validate_request(&payload)?;

    let result = pay_invoice(
        state.clone(),
        auth.business_id,
        path.into_inner(),
        headers.idempotency_key,
        payload,
    )
    .await?;

    match result {
        PaymentResult::Succeeded(body) => Ok(ApiResponse::success(body).ok()),
        PaymentResult::Pending(body) => Ok(ApiResponse::success(body).accepted()),
        PaymentResult::Failed(details) => Ok(ApiErrorResponse::with_details(
            "PAYMENT_FAILED",
            "Payment declined by processor",
            serde_json::to_value(details).unwrap(),
        )
        .with_status(StatusCode::UNPROCESSABLE_ENTITY)),
    }
}
