# syntax=docker/dockerfile:1

# ---- Build stage ----
FROM rust:1-slim-bookworm AS builder

# prost-build shells out to protoc to generate src/maps.proto bindings.
RUN apt-get update \
    && apt-get install -y --no-install-recommends protobuf-compiler \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY Cargo.toml Cargo.lock build.rs ./
COPY src ./src

# Cache the crate registry and compiled dependencies between builds. The binary
# is copied out of the cache mount so it survives into the runtime stage.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/app/target \
    cargo build --release --locked \
    && cp target/release/beatsaber-scrape-api /usr/local/bin/beatsaber-scrape-api

# ---- Runtime stage ----
FROM debian:bookworm-slim AS runtime

# reqwest (rustls) validates TLS certificates against the system trust store.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home app

COPY --from=builder /usr/local/bin/beatsaber-scrape-api /usr/local/bin/beatsaber-scrape-api

USER app

EXPOSE 3000

ENTRYPOINT ["beatsaber-scrape-api"]
