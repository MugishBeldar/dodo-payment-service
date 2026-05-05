use std::env;
use std::time::Duration;

use actix_web::{web, App, HttpResponse, HttpServer};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Deserialize, Validate)]
struct ChargeRequest {
    #[validate(length(min = 1))]
    card_token: String,
    #[serde(rename = "amount_cents")]
    #[validate(range(min = 1))]
    amount_cents: i64,
    #[serde(rename = "currency")]
    #[validate(length(min = 1))]
    currency: String,
    #[serde(rename = "reference")]
    #[validate(length(min = 1))]
    reference: String,
}

impl ChargeRequest {
    fn normalize(mut self) -> Self {
        self.card_token = self.card_token.trim().to_string();
        self.currency = self.currency.trim().to_string();
        self.reference = self.reference.trim().to_string();
        self
    }
}

#[derive(Debug, Serialize)]
struct ChargeSuccess {
    status: &'static str,
    psp_ref: String,
}

#[derive(Debug, Serialize)]
struct ChargeFailure {
    status: &'static str,
    code: &'static str,
}

async fn charge(payload: web::Json<ChargeRequest>) -> HttpResponse {
    let payload = payload.into_inner().normalize();
    if let Err(errors) = payload.validate() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "error": {
                "code": "BAD_REQUEST",
                "message": errors.to_string(),
            }
        }));
    }

    println!("Mock PSP /charge called with token: {}", payload.card_token);
    match payload.card_token.as_str() {
        "tok_success" => HttpResponse::Ok().json(ChargeSuccess {
            status: "succeeded",
            psp_ref: format!("psp_{}", Uuid::new_v4()),
        }),
        "tok_insufficient_funds" => HttpResponse::Ok().json(ChargeFailure {
            status: "failed",
            code: "insufficient_funds",
        }),
        "tok_card_declined" => HttpResponse::Ok().json(ChargeFailure {
            status: "failed",
            code: "card_declined",
        }),
        "tok_timeout" => {
            tokio::time::sleep(Duration::from_secs(30)).await;
            HttpResponse::Ok().json(ChargeSuccess {
                status: "succeeded",
                psp_ref: format!("psp_{}", Uuid::new_v4()),
            })
        }
        "tok_network_error" => {
            HttpResponse::InternalServerError().json(serde_json::json!({ "error": "internal server error" }))
        }
        _ => HttpResponse::Ok().json(ChargeFailure {
            status: "failed",
            code: "invalid_card_token",
        }),
    }
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    env_logger::builder().init();
    let port = env::var("PORT").ok().and_then(|s| s.parse().ok()).unwrap_or(9090);

    println!("Mock PSP starting on port {}", port);

    HttpServer::new(|| App::new().route("/charge", web::post().to(charge)))
        .bind(("0.0.0.0", port))?
        .run()
        .await
}
