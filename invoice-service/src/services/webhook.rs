use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use actix_web::web;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use sqlx::PgPool;
use sqlx::Row;
use uuid::Uuid;

use crate::AppState;

type HmacSha256 = Hmac<Sha256>;

const RETRY_SCHEDULE_SECONDS: [u64; 5] = [60, 300, 1800, 7200, 28800];

pub async fn enqueue_webhook_event(
    state: web::Data<AppState>,
    business_id: Uuid,
    event_type: &str,
    payload: serde_json::Value,
) {
    let endpoints = sqlx::query(
        r#"
        SELECT id, url, secret
        FROM webhook_endpoints
        WHERE business_id = $1 AND enabled = true
        "#,
    )
    .bind(business_id)
    .fetch_all(&state.pool)
    .await;

    let Ok(endpoints) = endpoints else {
        return;
    };

    for endpoint in endpoints {
        let endpoint_id: Uuid = endpoint.get("id");
        let url: String = endpoint.get("url");
        let secret: String = endpoint.get("secret");

        let inserted = sqlx::query(
            r#"
            INSERT INTO webhook_deliveries (endpoint_id, event_type, payload, status, attempt_count)
            VALUES ($1, $2, $3, 'pending', 0)
            RETURNING id
            "#,
        )
        .bind(endpoint_id)
        .bind(event_type)
        .bind(payload.clone())
        .fetch_one(&state.pool)
        .await;

        let Ok(delivery) = inserted else {
            continue;
        };

        let cloned_state = state.clone();
        let delivery_id: Uuid = delivery.get("id");
        let event_type = event_type.to_string();
        let payload = payload.clone();

        tokio::spawn(async move {
            deliver_with_retry(
                Arc::new(cloned_state.get_ref().clone()),
            delivery_id,
                url,
                secret,
                event_type,
                payload,
                0,
            )
            .await;
        });
    }
}

async fn deliver_with_retry(
    state: Arc<AppState>,
    delivery_id: Uuid,
    endpoint_url: String,
    secret: String,
    event_type: String,
    payload: serde_json::Value,
    attempt_count: usize,
) {
    let mut attempt_idx = attempt_count;
    let payload_text = payload.to_string();

    loop {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let signed_payload = format!("{timestamp}.{payload_text}");
        let signature = sign_payload(&secret, &signed_payload);

        let response = tokio::time::timeout(
            Duration::from_secs(state.config.webhook_timeout_secs),
            state
                .http_client
                .post(&endpoint_url)
                .header("Content-Type", "application/json")
                .header("X-Dodo-Event", &event_type)
                .header("X-Dodo-Timestamp", timestamp.to_string())
                .header("X-Dodo-Signature", format!("sha256={signature}"))
                .body(payload_text.clone())
                .send(),
        )
        .await;

        if let Ok(Ok(resp)) = response {
            if resp.status().is_success() {
                let _ = sqlx::query(
                    r#"
                    UPDATE webhook_deliveries
                    SET status = 'succeeded', attempt_count = $1, updated_at = NOW(), last_error = NULL
                    WHERE id = $2
                    "#,
                )
                .bind((attempt_idx as i32) + 1)
                .bind(delivery_id)
                .execute(&state.pool)
                .await;
                break;
            }
        }

        let exhausted = attempt_idx >= RETRY_SCHEDULE_SECONDS.len();
        if exhausted {
            let _ = sqlx::query(
                r#"
                UPDATE webhook_deliveries
                SET status = 'dead', attempt_count = $1, updated_at = NOW(),
                    last_error = 'retry limit reached', next_retry_at = NULL
                WHERE id = $2
                "#,
            )
            .bind((attempt_idx as i32) + 1)
            .bind(delivery_id)
            .execute(&state.pool)
            .await;
            break;
        }

        let wait = RETRY_SCHEDULE_SECONDS[attempt_idx];
        let _ = mark_retry_pending(&state.pool, delivery_id, (attempt_idx as i32) + 1, wait).await;
        tokio::time::sleep(Duration::from_secs(wait)).await;
        attempt_idx += 1;
    }
}

async fn mark_retry_pending(
    pool: &PgPool,
    delivery_id: Uuid,
    attempt_count: i32,
    wait_seconds: u64,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
        UPDATE webhook_deliveries
        SET status = 'failed',
            attempt_count = $1,
            next_retry_at = NOW() + ($2 || ' seconds')::interval,
            updated_at = NOW(),
            last_error = 'delivery failed'
        WHERE id = $3
        "#,
    )
    .bind(attempt_count)
    .bind(wait_seconds as i64)
    .bind(delivery_id)
    .execute(pool)
    .await?;

    Ok(())
}

fn sign_payload(secret: &str, payload: &str) -> String {
    let mut mac = HmacSha256::new_from_slice(secret.as_bytes()).unwrap();
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}
