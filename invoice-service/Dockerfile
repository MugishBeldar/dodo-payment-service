FROM rust:1.88 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin invoice-service

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libpq5 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/invoice-service /usr/local/bin/
CMD ["invoice-service"]
