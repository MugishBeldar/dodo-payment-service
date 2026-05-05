mod auth;
mod config;
mod db;
mod errors;
mod handlers;
mod models;
mod response;
mod services;
mod validation;
#[cfg(test)]
mod tests;

use actix_web::{middleware::Logger, web, App, HttpResponse, HttpServer};
use config::Config;
use serde::Serialize;
use sqlx::PgPool;

use crate::errors::AppError;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub config: Config,
    pub http_client: reqwest::Client,
}

#[derive(Serialize)]
struct HealthResponse {
    status: &'static str,
    version: &'static str,
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(HealthResponse {
        status: "ok",
        version: "0.1.0",
    })
}

fn json_error_handler(err: actix_web::error::JsonPayloadError, _req: &actix_web::HttpRequest) -> actix_web::Error {
    let message = if err.to_string().contains("EOF while parsing a value") {
        "Request body is required and must be valid JSON".to_string()
    } else {
        format!("Invalid JSON body: {err}")
    };

    AppError::BadRequest(message).into()
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenvy::dotenv().ok();
    env_logger::init();

    let config = Config::from_env();
    let pool = db::new_pool(&config.database_url)
        .await
        .expect("failed to connect to postgres");

    let state = AppState {
        pool,
        config: config.clone(),
        http_client: reqwest::Client::new(),
    };

    HttpServer::new(move || {
        App::new()
            .wrap(Logger::default())
            .app_data(web::Data::new(state.clone()))
            .app_data(web::JsonConfig::default().error_handler(json_error_handler))
            .route("/health", web::get().to(health))
            .service(
                web::scope("/customers")
                    .route("", web::post().to(handlers::customers::create_customer))
                    .route("", web::get().to(handlers::customers::list_customers))
                    .route("/{id}", web::get().to(handlers::customers::get_customer)),
            )
            .service(
                web::scope("/invoices")
                    .route("", web::post().to(handlers::invoices::create_invoice))
                    .route("", web::get().to(handlers::invoices::list_invoices))
                    .route("/{id}", web::get().to(handlers::invoices::get_invoice))
                    .route("/{id}/pay", web::post().to(handlers::payments::pay_invoice_handler)),
            )
            .service(
                web::scope("/webhook-endpoints")
                    .route("", web::post().to(handlers::webhooks::create_webhook_endpoint)),
            )
    })
    .bind(("0.0.0.0", config.port))?
    .run()
    .await
}
