FROM node:22-alpine AS frontend-builder

WORKDIR /app/admin-ui
COPY admin-ui/package.json admin-ui/pnpm-lock.yaml ./
RUN npm install -g pnpm && pnpm install --frozen-lockfile --ignore-scripts
COPY admin-ui ./
RUN pnpm build

FROM rust:1.92-alpine AS chef
RUN apk add --no-cache musl-dev openssl-dev openssl-libs-static && \
    cargo install cargo-chef --locked
WORKDIR /app

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
ENV CARGO_PROFILE_RELEASE_LTO=off
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY migrations ./migrations
COPY --from=frontend-builder /app/admin-ui/dist /app/admin-ui/dist
RUN cargo build --release

FROM alpine:3.21

RUN apk add --no-cache ca-certificates python3 py3-pip && \
    pip3 install --no-cache-dir --break-system-packages \
        curl_cffi cbor2 jwcrypto python-dotenv

WORKDIR /app
COPY --from=builder /app/target/release/kiro-rs /app/kiro-rs
COPY scripts/kiro_register.py /app/scripts/kiro_register.py

VOLUME ["/app/config"]

EXPOSE 8990

CMD ["./kiro-rs", "-c", "/app/config/config.json"]
