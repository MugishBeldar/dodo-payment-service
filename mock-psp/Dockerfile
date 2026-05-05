FROM rust:1.88 AS builder
WORKDIR /app
COPY . .
RUN cargo build --release --bin mock-psp

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/mock-psp /usr/local/bin/
CMD ["mock-psp"]
