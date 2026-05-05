use actix_web::{web, HttpResponse};
use serde::Deserialize;
use sqlx::Row;
use uuid::Uuid;
use validator::Validate;

use crate::auth::AuthenticatedBusiness;
use crate::errors::AppError;
use crate::models::invoice::{
    CreateInvoiceRequest, Invoice, InvoiceLineItem, InvoiceWithLineItems,
};
use crate::response::{ApiResponse, ListData};
use crate::services::webhook::enqueue_webhook_event;
use crate::validation::validate_request;
use crate::AppState;

#[derive(Deserialize, Validate)]
pub struct InvoiceListQuery {
    #[validate(custom(function = "crate::validation::validate_invoice_state"))]
    pub state: Option<String>,
}

impl InvoiceListQuery {
    fn normalize(mut self) -> Self {
        self.state = self.state.map(|value| value.trim().to_lowercase());
        self
    }
}

pub async fn create_invoice(
    state: web::Data<AppState>,
    auth: AuthenticatedBusiness,
    payload: web::Json<CreateInvoiceRequest>,
) -> Result<HttpResponse, AppError> {
    let payload = payload.into_inner().normalize();
    validate_request(&payload)?;

    let mut tx = state.pool.begin().await?;

    let customer_exists = sqlx::query_scalar::<_, i64>(
        r#"SELECT COUNT(1) FROM customers WHERE id = $1 AND business_id = $2"#,
    )
    .bind(payload.customer_id)
    .bind(auth.business_id)
    .fetch_one(&mut *tx)
    .await?;

    if customer_exists == 0 {
        return Err(AppError::NotFound("Customer not found".to_string()));
    }

    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        INSERT INTO invoices (business_id, customer_id, state, due_date, description)
        VALUES ($1, $2, 'open', $3, $4)
        RETURNING id, business_id, customer_id, state::text AS state, total_cents,
                  due_date, description, created_at, updated_at
        "#,
    )
    .bind(auth.business_id)
    .bind(payload.customer_id)
    .bind(payload.due_date)
    .bind(&payload.description)
    .fetch_one(&mut *tx)
    .await?;

    let mut total_cents: i64 = 0;
    let mut created_items: Vec<InvoiceLineItem> = Vec::with_capacity(payload.line_items.len());

    for li in &payload.line_items {
        let item = sqlx::query_as::<_, InvoiceLineItem>(
            r#"
            INSERT INTO invoice_line_items (invoice_id, description, quantity, unit_amount_cents)
            VALUES ($1, $2, $3, $4)
            RETURNING id, invoice_id, description, quantity, unit_amount_cents, line_total_cents
            "#,
        )
        .bind(invoice.id)
        .bind(&li.description)
        .bind(li.quantity)
        .bind(li.unit_amount_cents)
        .fetch_one(&mut *tx)
        .await?;

        total_cents += item.line_total_cents;
        created_items.push(item);
    }

    let invoice = sqlx::query_as::<_, Invoice>(
        r#"
        UPDATE invoices
        SET total_cents = $1, updated_at = NOW()
        WHERE id = $2
        RETURNING id, business_id, customer_id, state::text AS state, total_cents,
                  due_date, description, created_at, updated_at
        "#,
    )
    .bind(total_cents)
    .bind(invoice.id)
    .fetch_one(&mut *tx)
    .await?;

    tx.commit().await?;

    enqueue_webhook_event(
        state.clone(),
        auth.business_id,
        "invoice.created",
        serde_json::json!({
            "event": "invoice.created",
            "invoice_id": invoice.id,
            "business_id": auth.business_id,
            "total_cents": invoice.total_cents,
            "state": invoice.state,
        }),
    )
    .await;

    Ok(ApiResponse::success(InvoiceWithLineItems {
        invoice,
        line_items: created_items,
    })
    .created())
}

pub async fn get_invoice(
    state: web::Data<AppState>,
    auth: AuthenticatedBusiness,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let invoice_id = path.into_inner();
    let rows = sqlx::query(
        r#"
        SELECT
            i.id,
            i.business_id,
            i.customer_id,
            i.state::text AS state,
            i.total_cents,
            i.due_date,
            i.description,
            i.created_at,
            i.updated_at,
            li.id AS line_item_id,
            li.invoice_id AS line_item_invoice_id,
            li.description AS line_item_description,
            li.quantity AS line_item_quantity,
            li.unit_amount_cents AS line_item_unit_amount_cents,
            li.line_total_cents AS line_item_total_cents
        FROM invoices i
        LEFT JOIN invoice_line_items li ON li.invoice_id = i.id
        WHERE i.id = $1 AND i.business_id = $2
        ORDER BY li.id
        "#,
    )
    .bind(invoice_id)
    .bind(auth.business_id)
    .fetch_all(&state.pool)
    .await?;

    if rows.is_empty() {
        return Err(AppError::NotFound("Invoice not found".to_string()));
    }

    let first = &rows[0];
    let invoice = Invoice {
        id: first.try_get("id")?,
        business_id: first.try_get("business_id")?,
        customer_id: first.try_get("customer_id")?,
        state: first.try_get("state")?,
        total_cents: first.try_get("total_cents")?,
        due_date: first.try_get("due_date")?,
        description: first.try_get("description")?,
        created_at: first.try_get("created_at")?,
        updated_at: first.try_get("updated_at")?,
    };

    let mut line_items = Vec::new();
    for row in rows {
        let line_item_id: Option<Uuid> = row.try_get("line_item_id")?;
        if let Some(id) = line_item_id {
            line_items.push(InvoiceLineItem {
                id,
                invoice_id: row.try_get("line_item_invoice_id")?,
                description: row.try_get("line_item_description")?,
                quantity: row.try_get("line_item_quantity")?,
                unit_amount_cents: row.try_get("line_item_unit_amount_cents")?,
                line_total_cents: row.try_get("line_item_total_cents")?,
            });
        }
    }

    Ok(ApiResponse::success(InvoiceWithLineItems { invoice, line_items }).ok())
}

pub async fn list_invoices(
    state: web::Data<AppState>,
    auth: AuthenticatedBusiness,
    query: web::Query<InvoiceListQuery>,
) -> Result<HttpResponse, AppError> {
    let query = query.into_inner().normalize();
    validate_request(&query)?;

    let invoices = if let Some(state_filter) = &query.state {
        sqlx::query_as::<_, Invoice>(
            r#"
            SELECT id, business_id, customer_id, state::text AS state, total_cents,
                   due_date, description, created_at, updated_at
            FROM invoices
            WHERE business_id = $1 AND state::text = $2
            ORDER BY created_at DESC
            "#,
        )
        .bind(auth.business_id)
        .bind(state_filter)
        .fetch_all(&state.pool)
        .await?
    } else {
        sqlx::query_as::<_, Invoice>(
            r#"
            SELECT id, business_id, customer_id, state::text AS state, total_cents,
                   due_date, description, created_at, updated_at
            FROM invoices
            WHERE business_id = $1
            ORDER BY created_at DESC
            "#,
        )
        .bind(auth.business_id)
        .fetch_all(&state.pool)
        .await?
    };

    let total = invoices.len();
    let list_data = ListData::new(invoices, total);
    Ok(ApiResponse::success(list_data).ok())
}
