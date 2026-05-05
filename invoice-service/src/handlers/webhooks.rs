use actix_web::{web, HttpResponse};
use rand::RngCore;

use crate::auth::AuthenticatedBusiness;
use crate::errors::AppError;
use crate::models::webhook::{CreateWebhookEndpointRequest, WebhookEndpoint};
use crate::response::ApiResponse;
use crate::validation::validate_request;
use crate::AppState;

pub async fn create_webhook_endpoint(
    state: web::Data<AppState>,
    auth: AuthenticatedBusiness,
    payload: web::Json<CreateWebhookEndpointRequest>,
) -> Result<HttpResponse, AppError> {
    let payload = payload.into_inner().normalize();
    validate_request(&payload)?;

    let mut random = [0_u8; 32];
    rand::thread_rng().fill_bytes(&mut random);
    let secret = format!("whsec_{}", hex::encode(random));

    let endpoint = sqlx::query_as::<_, WebhookEndpoint>(
        r#"
        INSERT INTO webhook_endpoints (business_id, url, secret)
        VALUES ($1, $2, $3)
        RETURNING id, business_id, url, secret, enabled, created_at
        "#,
    )
    .bind(auth.business_id)
    .bind(&payload.url)
    .bind(&secret)
    .fetch_one(&state.pool)
    .await?;

    Ok(ApiResponse::success(endpoint).created())
}
