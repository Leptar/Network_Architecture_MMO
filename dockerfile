# Stage 1 - Compilation
FROM rust:1.87 AS builder

WORKDIR /app

# Copier tout le workspace
COPY Cargo.toml Cargo.lock ./
COPY dedicated_server/ ./dedicated_server/
COPY shared/ ./shared/
COPY orchestrator/ ./orchestrator/
COPY gatekeeper/ ./gatekeeper/


# Compiler uniquement le dedicated_server
RUN cargo build --release -p dedicated_server

# Stage 2 - Image finale
FROM debian:bookworm-slim

WORKDIR /app

# Copier seulement le binaire compilé
COPY --from=builder /app/target/release/dedicated_server .

CMD ["./dedicated_server"]