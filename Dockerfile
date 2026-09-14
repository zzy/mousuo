FROM rust:latest AS builder

RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake perl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /build
COPY . .

RUN sed -i 's|path = "../tmp/topcoat/crates/topcoat"|git = "https://github.com/tokio-rs/topcoat"|g' Cargo.toml

RUN cargo test --release

RUN cargo build --release \
    && cargo install --git https://github.com/tokio-rs/topcoat topcoat-cli \
    && topcoat asset bundle --release

FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ffmpeg \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /build/target/release/mousuo /app/
COPY --from=builder /build/target/release/assets /app/assets

WORKDIR /app
ENV HOST=0.0.0.0
ENV PORT=7800
CMD ["./mousuo"]
