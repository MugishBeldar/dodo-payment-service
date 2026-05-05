use actix_web::{dev::Payload, web, Error, FromRequest, HttpRequest};
use futures_util::future::LocalBoxFuture;
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::errors::AppError;
use crate::AppState;

#[derive(Debug, Clone)]
pub struct AuthenticatedBusiness {
    pub business_id: Uuid,
}

impl FromRequest for AuthenticatedBusiness {
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        let app_state = req.app_data::<web::Data<AppState>>().cloned();
        let auth_header = req
            .headers()
            .get("Authorization")
            .and_then(|v| v.to_str().ok())
            .map(|v| v.to_string());

        Box::pin(async move {
            let state = app_state.ok_or_else(|| {
                let err: Error = AppError::Unauthorized.into();
                err
            })?;
            let token = extract_bearer(auth_header.as_deref())
                .ok_or_else(|| {
                    let err: Error = AppError::Unauthorized.into();
                    err
                })?;
            let business_id = lookup_business_id(&state.pool, token)
                .await
                .map_err(|e| {
                    let err: Error = e.into();
                    err
                })?;

            Ok(Self {
                business_id,
            })
        })
    }
}

fn extract_bearer(header: Option<&str>) -> Option<&str> {
    let value = header?;
    value.strip_prefix("Bearer ")
}

async fn lookup_business_id(pool: &PgPool, raw_key: &str) -> Result<Uuid, AppError> {
    let mut hasher = Sha256::new();
    hasher.update(raw_key.as_bytes());
    let key_hash = hex::encode(hasher.finalize());

    let row = sqlx::query_scalar::<_, Uuid>(
        r#"
        SELECT business_id
        FROM api_keys
        WHERE key_hash = $1 AND revoked_at IS NULL
        "#,
    )
    .bind(key_hash)
    .fetch_optional(pool)
    .await?;

    match row {
        Some(id) => Ok(id),
        None => Err(AppError::Unauthorized),
    }
}
