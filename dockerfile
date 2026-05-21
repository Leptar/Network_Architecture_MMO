# Stage 1 - Compilation
FROM rust:1.95 AS builder

WORKDIR /app

# Copier tout le workspace
COPY Cargo.toml Cargo.lock ./
COPY dedicated_server/ ./dedicated_server/
COPY shared/ ./shared/
COPY orchestrator/ ./orchestrator/
COPY gatekeeper/ ./gatekeeper/

# Installer les dépendances nécessaires pour la compilation
RUN apt-get update && apt-get install -y clang libclang-dev cmake protobuf-compiler \
    && rustup component add rustfmt

# Compiler uniquement le dedicated_server
RUN cargo build --release -p dedicated_server

# Stage 2 - Image finale
FROM debian:trixie-slim

WORKDIR /app

# Installer les dépendances nécessaires pour exécuter le binaire
RUN apt-get update && apt-get install -y libprotobuf-dev && rm -rf /var/lib/apt/lists/*

# Copier seulement le binaire compilé
COPY --from=builder /app/target/release/dedicated_server .

CMD ["./dedicated_server"]