# dodo-payment-service# Explanation video link:- 
https://drive.google.com/file/d/1c1g7GBPaxSucY_DWrhd_mC26dPw53HnN/view?usp=drivesdk

# Run Guide

This guide shows how to run the Dodo Payments assignment project.

## 1) Prerequisites

Install:
- Docker + Docker Compose

Check versions:
- `docker --version`
- `docker compose version`

## 2) Run Everything with Docker (recommended)

From project root:

1. Build and start all services:
   - `docker compose up --build`

2. Start in background:
   - `docker compose up --build -d`

3. View logs:
   - `docker compose logs -f`
   - `docker compose logs -f invoice-service`
   - `docker compose logs -f mock-psp`
   - `docker compose logs -f db`

4. Stop services:
   - `docker compose down`

5. Stop and reset database volume:
   - `docker compose down -v`

## 4) Service Endpoints

- Invoice Service: `http://localhost:8080`
- Mock PSP: `http://localhost:9090`
- PostgreSQL: `localhost:5432`

Health check:
- `GET http://localhost:8080/health`

## 5) Default Database Settings

- Database: `dodo_payments`
- User: `dodo_user`
- Password: `dodo_password`
- Host (inside docker network): `db`

## 6) API Authentication
Use header on protected endpoints:
- `Authorization: Bearer dp_test_supersecretkey123`
Payment endpoint also requires:
- `Idempotency-Key: <unique-value>`

## 7) Quick Smoke Test (curl)
### Create customer
`curl -X POST http://localhost:8080/customers \
  -H "Authorization: Bearer dp_test_supersecretkey123" \
  -H "Content-Type: application/json" \
  -d '{"name":"Priya Sharma","email":"priya@example.com"}'`
### Create invoice
`curl -X POST http://localhost:8080/invoices \
  -H "Authorization: Bearer dp_test_supersecretkey123" \
  -H "Content-Type: application/json" \
  -d '{
    "customer_id":"<REPLACE_CUSTOMER_ID>",
    "due_date":"2026-05-30",
    "description":"Consulting services",
    "line_items":[
      {"description":"Backend dev","quantity":2,"unit_amount_cents":10000}
    ]
  }'`
### Pay invoice (success)

`curl -X POST http://localhost:8080/invoices/<REPLACE_INVOICE_ID>/pay \
  -H "Authorization: Bearer dp_test_supersecretkey123" \
  -H "Content-Type: application/json" \
  -H "Idempotency-Key: $(uuidgen)" \
  -d '{"card_token":"tok_success"}'`
  
## 8) Build and Test

- `cargo check`
- `cargo test`



