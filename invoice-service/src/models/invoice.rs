use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct Invoice {
    pub id: Uuid,
    pub business_id: Uuid,
    pub customer_id: Uuid,
    pub state: String,
    pub total_cents: i64,
    pub due_date: NaiveDate,
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct InvoiceLineItem {
    pub id: Uuid,
    pub invoice_id: Uuid,
    pub description: String,
    pub quantity: i32,
    pub unit_amount_cents: i64,
    pub line_total_cents: i64,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateLineItemRequest {
    #[validate(custom(function = "crate::validation::validate_non_blank"))]
    pub description: String,
    #[validate(range(min = 1))]
    pub quantity: i32,
    #[validate(range(min = 1))]
    pub unit_amount_cents: i64,
}

#[derive(Debug, Deserialize, Serialize, Validate)]
pub struct CreateInvoiceRequest {
    pub customer_id: Uuid,
    pub due_date: NaiveDate,
    pub description: Option<String>,
    #[validate(length(min = 1))]
    #[validate(nested)]
    pub line_items: Vec<CreateLineItemRequest>,
}

impl CreateInvoiceRequest {
    pub fn normalize(mut self) -> Self {
        self.description = self.description.map(|value| value.trim().to_string());
        self.line_items = self
            .line_items
            .into_iter()
            .map(CreateLineItemRequest::normalize)
            .collect();
        self
    }
}

impl CreateLineItemRequest {
    pub fn normalize(mut self) -> Self {
        self.description = self.description.trim().to_string();
        self
    }
}

#[derive(Debug, Serialize)]
pub struct InvoiceWithLineItems {
    #[serde(flatten)]
    pub invoice: Invoice,
    pub line_items: Vec<InvoiceLineItem>,
}
