use actix_web::{web, HttpResponse};
use uuid::Uuid;

use crate::auth::AuthenticatedBusiness;
use crate::errors::AppError;
use crate::models::customer::{CreateCustomerRequest, Customer};
use crate::response::{ApiResponse, ListData};
use crate::validation::validate_request;
use crate::AppState;

pub async fn create_customer(
    state: web::Data<AppState>,
    auth: AuthenticatedBusiness,
    payload: web::Json<CreateCustomerRequest>,
) -> Result<HttpResponse, AppError> {
    let payload = payload.into_inner().normalize();
    validate_request(&payload)?;

    let row = sqlx::query_as::<_, Customer>(
        r#"
        INSERT INTO customers (business_id, name, email)
        VALUES ($1, $2, $3)
        RETURNING id, business_id, name, email, created_at
        "#,
    )
    .bind(auth.business_id)
    .bind(&payload.name)
    .bind(&payload.email)
    .fetch_one(&state.pool)
    .await;

    match row {
        Ok(customer) => Ok(ApiResponse::success(customer).created()),
        Err(sqlx::Error::Database(db_err)) if db_err.code().as_deref() == Some("23505") => {
            Err(AppError::Conflict("A customer with this email already exists".to_string()))
        }
        Err(err) => Err(AppError::from(err)),
    }
}

pub async fn get_customer(
    state: web::Data<AppState>,
    auth: AuthenticatedBusiness,
    path: web::Path<Uuid>,
) -> Result<HttpResponse, AppError> {
    let customer = sqlx::query_as::<_, Customer>(
        r#"
        SELECT id, business_id, name, email, created_at
        FROM customers
        WHERE id = $1 AND business_id = $2
        "#,
    )
    .bind(path.into_inner())
    .bind(auth.business_id)
    .fetch_optional(&state.pool)
    .await?;

    customer
        .map(|c| ApiResponse::success(c).ok())
        .ok_or_else(|| AppError::NotFound("Customer not found".to_string()))
}

pub async fn list_customers(
    state: web::Data<AppState>,
    auth: AuthenticatedBusiness,
) -> Result<HttpResponse, AppError> {
    let customers = sqlx::query_as::<_, Customer>(
        r#"
        SELECT id, business_id, name, email, created_at
        FROM customers
        WHERE business_id = $1
        ORDER BY created_at DESC
        "#,
    )
    .bind(auth.business_id)
    .fetch_all(&state.pool)
    .await?;

    let total = customers.len();
    let list_data = ListData::new(customers, total);
    Ok(ApiResponse::success(list_data).ok())
}
