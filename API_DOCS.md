# Dodo Payments API Docs (Short & Practical)

This README is a quick API reference with:
- Endpoint-wise curl examples
- Request parameters (required/optional)
- Success response first, then error responses
- Short reason (`why`) for each variation

---

## Base URLs

| Service         | URL                      |
|-----------------|--------------------------|
| Invoice Service | `http://localhost:8080`  |
| Mock PSP        | `http://localhost:9090`  |

## Common Headers

| Header          | Value                                      | Used On              |
|-----------------|--------------------------------------------|----------------------|
| `Authorization` | `Bearer dp_test_supersecretkey123`         | All protected routes |
| `Content-Type`  | `application/json`                         | POST routes          |

## Standard Response Shapes

**Success — single object**
```json
{
  "status": "success",
  "data": {}
}
```

**Success — list**
```json
{
  "status": "success",
  "data": {
    "items": [{}, {}],
    "total": 2
  }
}
```

**Error**
```json
{
  "status": "error",
  "error": {
    "code": "BAD_REQUEST",
    "message": "...",
    "details": null
  }
}
```

---

## 1) GET /health

No request body required.

### ✅ Health check (success)

```bash
curl --location 'http://localhost:8080/health'
```

```json
{ "status": "ok", "version": "0.1.0" }
```

> **Why:** Service liveness check.

---

## 2) POST /customers

### Parameters

| Field   | Type   | Required | Notes                      |
|---------|--------|----------|----------------------------|
| `name`  | string | Yes      | Non-empty                  |
| `email` | string | Yes      | Non-empty, valid email     |

---

### ✅ Create customer (success)

```bash
curl --location --request POST 'http://localhost:8080/customers' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Content-Type: application/json' \
  --data '{"name":"Priya Sharma","email":"priya@example.com"}'
```

```json
{
  "status": "success",
  "data": {
    "id": "147920e8-4e32-459e-a714-881862f73fea",
    "business_id": "00000000-0000-0000-0000-000000000001",
    "name": "Priya Sharma",
    "email": "priya@example.com",
    "created_at": "2026-05-05T07:11:12.595653Z"
  }
}
```

> **Why:** Creates a customer in current business scope.

---

### ❌ Create customer (validation error)

```bash
curl --location --request POST 'http://localhost:8080/customers' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Content-Type: application/json' \
  --data '{"name":"","email":""}'
```

```json
{
  "status": "error",
  "error": {
    "code": "BAD_REQUEST",
    "message": "email must be a valid email address, email is required, name is required"
  }
}
```

> **Why:** Validation failure on empty fields.

---

### ❌ Create customer (empty body error)

```bash
curl --location --request POST 'http://localhost:8080/customers' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Content-Type: application/json' \
  --data ''
```

```json
{
  "status": "error",
  "error": {
    "code": "BAD_REQUEST",
    "message": "Request body is required and must be valid JSON"
  }
}
```

> **Why:** Body missing or invalid JSON.

---

### ❌ Create customer (auth error)

```bash
curl --location --request POST 'http://localhost:8080/customers' \
  --header 'Authorization: Bearer invalid_key' \
  --header 'Content-Type: application/json' \
  --data '{"name":"A","email":"a@b.com"}'
```

```json
{
  "status": "error",
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Invalid or missing API key"
  }
}
```

> **Why:** Invalid bearer token.

---

## 3) GET /customers

No request body required.

### ✅ List customers (success)

```bash
curl --location 'http://localhost:8080/customers' \
  --header 'Authorization: Bearer dp_test_supersecretkey123'
```

```json
{
  "status": "success",
  "data": {
    "items": [
      {
        "id": "...",
        "business_id": "00000000-0000-0000-0000-000000000001",
        "name": "Priya Sharma",
        "email": "priya@example.com",
        "created_at": "2026-05-05T07:11:12.595653Z"
      }
    ],
    "total": 1
  }
}
```

> **Why:** Returns all customers for the authenticated business.

---

### ❌ List customers (auth error)

```bash
curl --location 'http://localhost:8080/customers' \
  --header 'Authorization: Bearer invalid_key'
```

```json
{
  "status": "error",
  "error": {
    "code": "UNAUTHORIZED",
    "message": "Invalid or missing API key"
  }
}
```

> **Why:** Invalid token.

---

## 4) GET /customers/{id}

No request body required.

### ✅ Get customer (success)

```bash
curl --location 'http://localhost:8080/customers/{customer_id}' \
  --header 'Authorization: Bearer dp_test_supersecretkey123'
```

```json
{
  "status": "success",
  "data": {
    "id": "{customer_id}",
    "business_id": "00000000-0000-0000-0000-000000000001",
    "name": "Priya Sharma",
    "email": "priya@example.com",
    "created_at": "2026-05-05T07:11:12.595653Z"
  }
}
```

> **Why:** Fetches one customer by ID within the current tenant.

---

### ❌ Get customer (not found)

```bash
curl --location 'http://localhost:8080/customers/00000000-0000-0000-0000-000000000999' \
  --header 'Authorization: Bearer dp_test_supersecretkey123'
```

```json
{
  "status": "error",
  "error": {
    "code": "NOT_FOUND",
    "message": "Customer not found"
  }
}
```

> **Why:** Customer ID not in current tenant.

---

## 5) POST /invoices

### Parameters

| Field                            | Type             | Required | Notes                  |
|----------------------------------|------------------|----------|------------------------|
| `customer_id`                    | uuid             | Yes      | Must belong to same business |
| `due_date`                       | string (YYYY-MM-DD) | Yes   | Invoice due date       |
| `description`                    | string           | No       | Optional description   |
| `line_items`                     | array            | Yes      | Min length = 1         |
| `line_items[].description`       | string           | Yes      | Non-empty              |
| `line_items[].quantity`          | integer          | Yes      | Min = 1                |
| `line_items[].unit_amount_cents` | integer          | Yes      | Min = 1                |

---

### ✅ Create invoice (success)

```bash
curl --location --request POST 'http://localhost:8080/invoices' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Content-Type: application/json' \
  --data '{
    "customer_id": "{customer_id}",
    "due_date": "2026-05-20",
    "description": "May consulting",
    "line_items": [
      { "description": "Backend work", "quantity": 2, "unit_amount_cents": 5000 }
    ]
  }'
```

```json
{
  "status": "success",
  "data": {
    "id": "{invoice_id}",
    "business_id": "00000000-0000-0000-0000-000000000001",
    "customer_id": "{customer_id}",
    "state": "open",
    "total_cents": 10000,
    "due_date": "2026-05-20",
    "description": "May consulting",
    "created_at": "...",
    "updated_at": "...",
    "line_items": [
      {
        "id": "...",
        "invoice_id": "{invoice_id}",
        "description": "Backend work",
        "quantity": 2,
        "unit_amount_cents": 5000,
        "line_total_cents": 10000
      }
    ]
  }
}
```

> **Why:** Creates invoice + line items in one transaction.

---

### ❌ Create invoice (validation error — empty line_items)

```bash
curl --location --request POST 'http://localhost:8080/invoices' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Content-Type: application/json' \
  --data '{"customer_id":"{customer_id}","due_date":"2026-05-20","line_items":[]}'
```

```json
{
  "status": "error",
  "error": {
    "code": "BAD_REQUEST",
    "message": "line_items has an invalid length"
  }
}
```

> **Why:** At least one line item is required.

---

### ❌ Create invoice (unknown customer)

```bash
curl --location --request POST 'http://localhost:8080/invoices' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Content-Type: application/json' \
  --data '{
    "customer_id": "00000000-0000-0000-0000-000000000999",
    "due_date": "2026-05-20",
    "line_items": [{ "description": "x", "quantity": 1, "unit_amount_cents": 100 }]
  }'
```

```json
{
  "status": "error",
  "error": {
    "code": "NOT_FOUND",
    "message": "Customer not found"
  }
}
```

> **Why:** Tenant-safe customer existence check.

---

## 6) GET /invoices

### Query Parameters

| Param   | Type   | Required | Notes                                              |
|---------|--------|----------|----------------------------------------------------|
| `state` | string | No       | One of: `draft`, `open`, `paid`, `void`, `uncollectible` |

---

### ✅ List invoices (success)

```bash
curl --location 'http://localhost:8080/invoices' \
  --header 'Authorization: Bearer dp_test_supersecretkey123'
```

```json
{
  "status": "success",
  "data": {
    "items": [{ "id": "{invoice_id}", "state": "open", "total_cents": 10000 }],
    "total": 1
  }
}
```

> **Why:** Returns all invoices for the current tenant.

---

### ✅ List invoices filtered by state

```bash
curl --location 'http://localhost:8080/invoices?state=paid' \
  --header 'Authorization: Bearer dp_test_supersecretkey123'
```

```json
{
  "status": "success",
  "data": {
    "items": [{ "id": "{invoice_id}", "state": "paid" }],
    "total": 1
  }
}
```

> **Why:** Returns invoices matching the given state.

---

### ❌ List invoices (invalid state query param)

```bash
curl --location 'http://localhost:8080/invoices?state=invalid_state' \
  --header 'Authorization: Bearer dp_test_supersecretkey123'
```

```json
{
  "status": "error",
  "error": {
    "code": "BAD_REQUEST",
    "message": "state must be one of: draft, open, paid, void, uncollectible"
  }
}
```

> **Why:** Enforced state enum validation.

---

## 7) GET /invoices/{id}

No request body required.

### ✅ Get invoice (success)

```bash
curl --location 'http://localhost:8080/invoices/{invoice_id}' \
  --header 'Authorization: Bearer dp_test_supersecretkey123'
```

```json
{
  "status": "success",
  "data": {
    "id": "{invoice_id}",
    "business_id": "00000000-0000-0000-0000-000000000001",
    "customer_id": "{customer_id}",
    "state": "open",
    "total_cents": 10000,
    "due_date": "2026-05-20",
    "description": "May consulting",
    "created_at": "...",
    "updated_at": "...",
    "line_items": [
      {
        "id": "...",
        "invoice_id": "{invoice_id}",
        "description": "Backend work",
        "quantity": 2,
        "unit_amount_cents": 5000,
        "line_total_cents": 10000
      }
    ]
  }
}
```

> **Why:** Fetches invoice with all line items.

---

### ❌ Get invoice (not found)

```bash
curl --location 'http://localhost:8080/invoices/00000000-0000-0000-0000-000000000999' \
  --header 'Authorization: Bearer dp_test_supersecretkey123'
```

```json
{
  "status": "error",
  "error": {
    "code": "NOT_FOUND",
    "message": "Invoice not found"
  }
}
```

> **Why:** Missing or cross-tenant invoice ID.

---

## 8) POST /invoices/{id}/pay

### Parameters

| Field             | Where  | Type   | Required | Notes                                                                 |
|-------------------|--------|--------|----------|-----------------------------------------------------------------------|
| `card_token`      | body   | string | Yes      | `tok_success`, `tok_timeout`, `tok_insufficient_funds`, `tok_network_error` |
| `Idempotency-Key` | header | uuid   | Yes      | Same key = safe retry, prevents double charge                         |

---

### ✅ Pay invoice — approved (HTTP 200)

```bash
curl --location --request POST 'http://localhost:8080/invoices/{invoice_id}/pay' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Idempotency-Key: 550e8400-e29b-41d4-a716-446655440000' \
  --header 'Content-Type: application/json' \
  --data '{"card_token":"tok_success"}'
```

```json
{
  "status": "success",
  "data": {
    "payment_attempt_id": "...",
    "invoice_id": "{invoice_id}",
    "status": "succeeded",
    "psp_reference": "psp_12345",
    "amount_cents": 10000
  }
}
```

> **Why:** PSP approved the charge.

---

### ⏳ Pay invoice — pending (HTTP 202)

```bash
curl --location --request POST 'http://localhost:8080/invoices/{invoice_id}/pay' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Idempotency-Key: 550e8400-e29b-41d4-a716-446655440001' \
  --header 'Content-Type: application/json' \
  --data '{"card_token":"tok_timeout"}'
```

```json
{
  "status": "success",
  "data": {
    "payment_attempt_id": "...",
    "invoice_id": "{invoice_id}",
    "status": "pending",
    "message": "PSP timeout, status unknown. Retry later with same idempotency key."
  }
}
```

> **Why:** PSP timed out; outcome uncertain — retry with the same `Idempotency-Key`.

---

### ❌ Pay invoice — declined (HTTP 422)

```bash
curl --location --request POST 'http://localhost:8080/invoices/{invoice_id}/pay' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Idempotency-Key: 550e8400-e29b-41d4-a716-446655440002' \
  --header 'Content-Type: application/json' \
  --data '{"card_token":"tok_insufficient_funds"}'
```

```json
{
  "status": "error",
  "error": {
    "code": "PAYMENT_FAILED",
    "message": "Payment declined by processor",
    "details": {
      "failure_code": "insufficient_funds",
      "payment_attempt_id": "..."
    }
  }
}
```

> **Why:** PSP declined the charge.

---

### ❌ Pay invoice — missing idempotency header

```bash
curl --location --request POST 'http://localhost:8080/invoices/{invoice_id}/pay' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Content-Type: application/json' \
  --data '{"card_token":"tok_success"}'
```

```json
{
  "status": "error",
  "error": {
    "code": "BAD_REQUEST",
    "message": "Idempotency-Key header is required"
  }
}
```

> **Why:** `Idempotency-Key` is mandatory to prevent duplicate charges.

---

## 9) POST /webhook-endpoints

### Parameters

| Field | Type   | Required | Notes                             |
|-------|--------|----------|-----------------------------------|
| `url` | string | Yes      | Must start with `http://` or `https://` |

---

### ✅ Register webhook (success)

```bash
curl --location --request POST 'http://localhost:8080/webhook-endpoints' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Content-Type: application/json' \
  --data '{"url":"https://example.com/webhook"}'
```

```json
{
  "status": "success",
  "data": {
    "id": "...",
    "business_id": "00000000-0000-0000-0000-000000000001",
    "url": "https://example.com/webhook",
    "secret": "whsec_xxx",
    "enabled": true,
    "created_at": "..."
  }
}
```

> **Why:** Adds a destination for async event delivery.

---

### ❌ Register webhook (invalid URL)

```bash
curl --location --request POST 'http://localhost:8080/webhook-endpoints' \
  --header 'Authorization: Bearer dp_test_supersecretkey123' \
  --header 'Content-Type: application/json' \
  --data '{"url":"ftp://invalid"}'
```

```json
{
  "status": "error",
  "error": {
    "code": "BAD_REQUEST",
    "message": "url must start with http:// or https://"
  }
}
```

> **Why:** Only HTTP(S) endpoints are allowed.

---

## 10) POST /charge (Mock PSP — port 9090)

### Parameters

| Field          | Type    | Required | Notes                       |
|----------------|---------|----------|-----------------------------|
| `card_token`   | string  | Yes      | Drives mock PSP behavior    |
| `amount_cents` | integer | Yes      | Min = 1                     |
| `currency`     | string  | Yes      | Non-empty (e.g. `USD`)      |
| `reference`    | string  | Yes      | Invoice reference           |

---

### ✅ Mock charge (success)

```bash
curl --location --request POST 'http://localhost:9090/charge' \
  --header 'Content-Type: application/json' \
  --data '{"card_token":"tok_success","amount_cents":10000,"currency":"USD","reference":"inv_123"}'
```

```json
{ "status": "succeeded", "psp_ref": "psp_mock_xxx" }
```

> **Why:** Simulates PSP approval.

---

### ❌ Mock charge (declined)

```bash
curl --location --request POST 'http://localhost:9090/charge' \
  --header 'Content-Type: application/json' \
  --data '{"card_token":"tok_insufficient_funds","amount_cents":10000,"currency":"USD","reference":"inv_123"}'
```

```json
{ "status": "failed", "failure_code": "insufficient_funds" }
```

> **Why:** Simulates PSP decline.

---

### ❌ Mock charge (validation error)

```bash
curl --location --request POST 'http://localhost:9090/charge' \
  --header 'Content-Type: application/json' \
  --data '{"card_token":"","amount_cents":0,"currency":"","reference":""}'
```

```json
{ "error": "validation failed: card_token ... amount_cents ..." }
```

> **Why:** Request validation failure on the mock PSP.

---

## Quick Test Order

1. Create a customer
2. Create an invoice
3. Get / list the invoice
4. Pay the invoice (`tok_success`, `tok_insufficient_funds`, `tok_timeout`)
5. Register a webhook endpoint
6. Try one invalid request per endpoint to verify error shape