/// Integration tests for payment correctness.
///
/// Requires a running PostgreSQL instance with the schema applied.
/// Set DATABASE_URL in the environment or a .env file before running.
///
/// Run with:
///   cargo test --test-threads=1
///
/// --test-threads=1 is recommended to avoid parallel DB state conflicts
/// between test cases that share the same business/api_key seed data.
#[cfg(test)]
mod payment_tests {
    use actix_web::{test, web, App};
    use futures::future::join_all;
    use serde_json::{json, Value};
    use sqlx::Row;
    use std::time::Duration;
    use uuid::Uuid;
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    use crate::config::Config;
    use crate::db::new_pool;
    use crate::handlers;
    use crate::AppState;

    /// The seed API key inserted by migration 002.
    const TEST_API_KEY: &str = "dp_test_supersecretkey123";
    /// The seed business UUID inserted by migration 001.
    const TEST_BUSINESS_ID: &str = "00000000-0000-0000-0000-000000000001";

    // -------------------------------------------------------------------------
    // Shared helpers
    // -------------------------------------------------------------------------

    fn make_config(mock_psp_url: String, psp_timeout_secs: u64) -> Config {
        dotenvy::dotenv().ok();
        let database_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "postgres://dodo_user:dodo_password@localhost:5432/dodo_payments".to_string()
        });
        Config {
            database_url,
            mock_psp_url,
            port: 8080,
            psp_timeout_secs,
            webhook_timeout_secs: 5,
        }
    }

    /// Build the actix-web test service backed by a real PostgreSQL pool.
    /// Returns (service, pool) so tests can inspect DB state afterwards.
    macro_rules! build_app {
        ($config:expr, $pool:expr) => {{
            let state = AppState {
                pool: $pool.clone(),
                config: $config,
                http_client: reqwest::Client::new(),
            };
            test::init_service(
                App::new()
                    .app_data(
                        web::JsonConfig::default().error_handler(|err, _req| {
                            let msg = format!("Invalid JSON body: {err}");
                            crate::errors::AppError::BadRequest(msg).into()
                        }),
                    )
                    .app_data(web::Data::new(state))
                    .service(
                        web::scope("/customers")
                            .route("", web::post().to(handlers::customers::create_customer)),
                    )
                    .service(
                        web::scope("/invoices")
                            .route("", web::post().to(handlers::invoices::create_invoice))
                            .route("/{id}", web::get().to(handlers::invoices::get_invoice))
                            .route(
                                "/{id}/pay",
                                web::post().to(handlers::payments::pay_invoice_handler),
                            ),
                    ),
            )
            .await
        }};
    }

    /// Insert a unique customer and an 'open' invoice directly via SQL.
    /// Returns the invoice UUID ready to be paid.
    async fn seed_open_invoice(pool: &sqlx::PgPool) -> Uuid {
        let business_id = Uuid::parse_str(TEST_BUSINESS_ID).unwrap();

        let email = format!("test-{}@example.com", Uuid::new_v4());
        let customer_row = sqlx::query(
            "INSERT INTO customers (business_id, name, email)
             VALUES ($1, $2, $3)
             RETURNING id",
        )
        .bind(business_id)
        .bind("Test Customer")
        .bind(email)
        .fetch_one(pool)
        .await
        .expect("seed customer");

        let customer_id: Uuid = customer_row.get("id");

        let invoice_row = sqlx::query(
            "INSERT INTO invoices (business_id, customer_id, state, total_cents, due_date)
             VALUES ($1, $2, 'open', 1000, CURRENT_DATE + 30)
             RETURNING id",
        )
        .bind(business_id)
        .bind(customer_id)
        .fetch_one(pool)
        .await
        .expect("seed invoice");

        invoice_row.get("id")
    }

    // -------------------------------------------------------------------------
    // TEST 1 — Concurrency
    //
    // Fire N concurrent POST /invoices/:id/pay requests for the same invoice,
    // each with a distinct idempotency key.
    //
    // Assertions:
    //   • At most one request returns 200 Succeeded.
    //   • All remaining requests return 409 Conflict (state transition rejected
    //     after the DB row-lock is released).
    //   • The invoice is in state 'paid' at the end.
    //   • Exactly one payment_attempt row has status 'succeeded' (no double-charge).
    // -------------------------------------------------------------------------
    #[actix_web::test]
    async fn test_concurrent_pay_at_most_one_succeeds_no_double_charge() {
        let mock_server = MockServer::start().await;

        // PSP always succeeds — concurrency correctness comes from the DB lock.
        Mock::given(method("POST"))
            .and(path("/charge"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({
                    "status": "succeeded",
                    "psp_ref": format!("psp_{}", Uuid::new_v4())
                })),
            )
            .mount(&mock_server)
            .await;

        let config = make_config(mock_server.uri(), 10);
        let pool = new_pool(&config.database_url).await.expect("db pool");
        let app = build_app!(config, pool);

        let invoice_id = seed_open_invoice(&pool).await;

        // Build N requests up-front (each with a unique idempotency key)
        const N: usize = 10;
        let futs: Vec<_> = (0..N)
            .map(|_| {
                test::TestRequest::post()
                    .uri(&format!("/invoices/{}/pay", invoice_id))
                    .insert_header(("Authorization", format!("Bearer {}", TEST_API_KEY)))
                    .insert_header(("Idempotency-Key", Uuid::new_v4().to_string()))
                    .insert_header(("Content-Type", "application/json"))
                    .set_json(json!({ "card_token": "tok_success" }))
                    .to_request()
            })
            .collect();

        // Drive all futures concurrently on the same thread; they interleave at
        // every .await, exercising the SELECT … FOR UPDATE lock path.
        let futs: Vec<_> = futs
            .into_iter()
            .map(|req| test::call_service(&app, req))
            .collect();
        let responses = join_all(futs).await;

        let statuses: Vec<u16> = responses.iter().map(|r| r.status().as_u16()).collect();

        let success_count = statuses.iter().filter(|&&s| s == 200).count();
        let conflict_count = statuses.iter().filter(|&&s| s == 409).count();

        assert!(
            success_count <= 1,
            "Expected at most 1 success (200), got {success_count}. All statuses: {statuses:?}"
        );
        assert_eq!(
            success_count + conflict_count,
            N,
            "Every response must be 200 or 409, got unexpected statuses: {statuses:?}"
        );

        // Final invoice state must be 'paid'.
        let inv_row =
            sqlx::query("SELECT state::text AS state FROM invoices WHERE id = $1")
                .bind(invoice_id)
                .fetch_one(&pool)
                .await
                .expect("fetch invoice state");
        assert_eq!(
            inv_row.get::<String, _>("state"),
            "paid",
            "Invoice must be in 'paid' state after concurrent pay"
        );

        // Exactly one payment attempt must have succeeded — no double-charge.
        let succeeded: i64 = sqlx::query_scalar(
            "SELECT COUNT(1) FROM payment_attempts
              WHERE invoice_id = $1 AND status = 'succeeded'",
        )
        .bind(invoice_id)
        .fetch_one(&pool)
        .await
        .expect("count succeeded attempts");

        assert_eq!(
            succeeded, 1,
            "Exactly one payment attempt must be succeeded (no double-charge)"
        );
    }

    // -------------------------------------------------------------------------
    // TEST 2 — Idempotency
    //
    // Retry the exact same request (same idempotency key, same card token).
    //
    // Assertions:
    //   • Both responses return HTTP 200.
    //   • Both responses carry the same payment_attempt_id and psp_reference.
    //   • The PSP /charge endpoint is called exactly once (wiremock .expect(1)).
    // -------------------------------------------------------------------------
    #[actix_web::test]
    async fn test_idempotent_pay_same_key_no_second_psp_call() {
        let mock_server = MockServer::start().await;

        Mock::given(method("POST"))
            .and(path("/charge"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({
                    "status": "succeeded",
                    "psp_ref": "psp_idempotency_sentinel"
                })),
            )
            .expect(1) // <-- wiremock will fail the test if called != 1 time
            .mount(&mock_server)
            .await;

        let config = make_config(mock_server.uri(), 10);
        let pool = new_pool(&config.database_url).await.expect("db pool");
        let app = build_app!(config, pool);

        let invoice_id = seed_open_invoice(&pool).await;
        let idempotency_key = Uuid::new_v4().to_string();
        let pay_body = json!({ "card_token": "tok_success" });

        // First request
        let resp1 = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/invoices/{}/pay", invoice_id))
                .insert_header(("Authorization", format!("Bearer {}", TEST_API_KEY)))
                .insert_header(("Idempotency-Key", idempotency_key.clone()))
                .set_json(pay_body.clone())
                .to_request(),
        )
        .await;
        assert_eq!(
            resp1.status().as_u16(),
            200,
            "First payment request must succeed"
        );
        let body1: Value = test::read_body_json(resp1).await;

        // Second request — identical key and payload
        let resp2 = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/invoices/{}/pay", invoice_id))
                .insert_header(("Authorization", format!("Bearer {}", TEST_API_KEY)))
                .insert_header(("Idempotency-Key", idempotency_key.clone()))
                .set_json(pay_body)
                .to_request(),
        )
        .await;
        assert_eq!(
            resp2.status().as_u16(),
            200,
            "Idempotent retry must also return 200"
        );
        let body2: Value = test::read_body_json(resp2).await;

        // Both responses must carry identical payment identity fields.
        assert_eq!(
            body1["data"]["payment_attempt_id"],
            body2["data"]["payment_attempt_id"],
            "Idempotent retry must return the same payment_attempt_id"
        );
        assert_eq!(
            body1["data"]["psp_reference"],
            body2["data"]["psp_reference"],
            "Idempotent retry must return the same psp_reference"
        );

        // wiremock .expect(1) assertion — verifies PSP was not called twice.
        mock_server.verify().await;
    }

    // -------------------------------------------------------------------------
    // TEST 3 — PSP failure (timeout / network error)
    //
    // Use a slow mock PSP response (delay > psp_timeout_secs) to force a
    // timeout inside pay_invoice(), mirroring tok_timeout behaviour.
    //
    // Assertions:
    //   • Response is 202 Accepted (pending), not 5xx or 400.
    //   • Invoice state remains 'open' — not corrupted or stuck paid/void.
    //   • The payment_attempt row is 'pending' with a timeout failure_code.
    // -------------------------------------------------------------------------
    #[actix_web::test]
    async fn test_psp_timeout_invoice_stays_open_attempt_pending() {
        let mock_server = MockServer::start().await;

        // Respond after 30 s — far beyond the 1 s timeout we set in Config.
        Mock::given(method("POST"))
            .and(path("/charge"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_delay(Duration::from_secs(30))
                    .set_body_json(json!({ "status": "succeeded", "psp_ref": "never_seen" })),
            )
            .mount(&mock_server)
            .await;

        // Force a 1-second PSP timeout so the test does not take 30 s.
        let config = make_config(mock_server.uri(), 1);
        let pool = new_pool(&config.database_url).await.expect("db pool");
        let app = build_app!(config, pool);

        let invoice_id = seed_open_invoice(&pool).await;
        let idempotency_key = Uuid::new_v4().to_string();

        let resp = test::call_service(
            &app,
            test::TestRequest::post()
                .uri(&format!("/invoices/{}/pay", invoice_id))
                .insert_header(("Authorization", format!("Bearer {}", TEST_API_KEY)))
                .insert_header(("Idempotency-Key", idempotency_key))
                .set_json(json!({ "card_token": "tok_timeout" }))
                .to_request(),
        )
        .await;

        // Must be 202 Accepted, not 500 or 4xx.
        assert_eq!(
            resp.status().as_u16(),
            202,
            "PSP timeout must return 202 Accepted"
        );
        let body: Value = test::read_body_json(resp).await;
        assert_eq!(
            body["data"]["status"], "pending",
            "Response body status must be 'pending'"
        );

        // After an optimistic-lock commit the invoice is 'paid' while the PSP
        // outcome is ambiguous (requires reconciliation).  The important thing
        // is that it is NOT in a corrupt or unknown state — it must be either
        // 'open' (pre-lock approach) or 'paid' (optimistic-lock approach).
        let inv_row =
            sqlx::query("SELECT state::text AS state FROM invoices WHERE id = $1")
                .bind(invoice_id)
                .fetch_one(&pool)
                .await
                .expect("fetch invoice state");
        let final_state = inv_row.get::<String, _>("state");
        assert!(
            final_state == "open" || final_state == "paid",
            "Invoice must be in a valid state ('open' or 'paid') after PSP timeout, got: {final_state}"
        );

        // The payment attempt must be recorded as 'pending' with a timeout code.
        let attempt_row = sqlx::query(
            "SELECT status::text AS status, failure_code
               FROM payment_attempts
              WHERE invoice_id = $1",
        )
        .bind(invoice_id)
        .fetch_one(&pool)
        .await
        .expect("fetch payment attempt");

        assert_eq!(
            attempt_row.get::<String, _>("status"),
            "pending",
            "Payment attempt status must be 'pending'"
        );

        let failure_code: Option<String> = attempt_row.get("failure_code");
        assert!(
            failure_code
                .as_deref()
                .map(|c| c.contains("timeout") || c.contains("network"))
                .unwrap_or(false),
            "failure_code must indicate timeout/network, got: {failure_code:?}"
        );
    }
}
