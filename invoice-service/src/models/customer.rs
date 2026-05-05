use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Customer {
    pub id: Uuid,
    pub business_id: Uuid,
    pub name: String,
    pub email: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCustomerRequest {
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub name: String,
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[validate(email)]
    pub email: String,
}

impl CreateCustomerRequest {
    pub fn normalize(mut self) -> Self {
        self.name = self.name.trim().to_string();
        self.email = self.email.trim().to_string();
        self
    }
}
