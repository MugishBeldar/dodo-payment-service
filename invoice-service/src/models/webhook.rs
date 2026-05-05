use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct CreateWebhookEndpointRequest {
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[validate(custom(function = "crate::validation::validate_http_url"))]
    pub url: String,
}

impl CreateWebhookEndpointRequest {
    pub fn normalize(mut self) -> Self {
        self.url = self.url.trim().to_string();
        self
    }
}

#[derive(Debug, Serialize, FromRow, Clone)]
pub struct WebhookEndpoint {
    pub id: Uuid,
    pub business_id: Uuid,
    pub url: String,
    pub secret: String,
    pub enabled: bool,
    pub created_at: DateTime<Utc>,
}
