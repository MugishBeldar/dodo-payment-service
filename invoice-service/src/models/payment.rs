use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
pub struct PayInvoiceRequest {
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub card_token: String,
}

#[derive(Debug, Validate)]
pub struct PayInvoiceHeaders {
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    #[validate(custom(function = "crate::validation::validate_uuid_string"))]
    pub idempotency_key: String,
}

impl PayInvoiceRequest {
    pub fn normalize(mut self) -> Self {
        self.card_token = self.card_token.trim().to_string();
        self
    }
}

impl PayInvoiceHeaders {
    pub fn normalize(mut self) -> Self {
        self.idempotency_key = self.idempotency_key.trim().to_string();
        self
    }
}

#[derive(Debug, Serialize, Clone)]
pub struct PaymentSuccessResponse {
    pub payment_attempt_id: Uuid,
    pub invoice_id: Uuid,
    pub status: String,
    pub psp_reference: Option<String>,
    pub amount_cents: i64,
}

#[derive(Debug, Serialize, Clone)]
pub struct PaymentPendingResponse {
    pub payment_attempt_id: Uuid,
    pub invoice_id: Uuid,
    pub status: String,
    pub message: String,
}

