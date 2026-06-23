# syntax=docker/dockerfile:1.7
# Build stage using bookworm to match runtime stage glibc
FROM rust:1.96-slim-bookworm AS builder

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    mold \
    clang \
    && rm -rf /var/lib/apt/lists/*

ENV CARGO_BUILD_JOBS=4
ENV RUSTFLAGS="-C link-arg=-fuse-ld=mold"

WORKDIR /usr/src/app

# Stub workspace libraries to cache dependencies
# 1. validator-engine stub
RUN mkdir -p backend/validator/src && echo "pub fn dummy() {}" > backend/validator/src/lib.rs
COPY backend/validator/Cargo.toml ./backend/validator/

# 2. backend stub
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs
COPY backend/Cargo.toml ./backend/

# Pre-build dependencies
WORKDIR /usr/src/app/backend
RUN cargo build --release

# Copy real source code
WORKDIR /usr/src/app
RUN echo "Cache bust 3"
COPY backend/src ./backend/src
COPY backend/validator ./backend/validator

# Force Cargo to rebuild the stubbed targets since host files have older timestamps
RUN echo "Force cache bust 1" && touch backend/src/main.rs backend/validator/src/lib.rs

# Build actual binary
WORKDIR /usr/src/app/backend
RUN cargo build --release

# Copy and build smart contract submit_complete CLI tool
WORKDIR /usr/src/app
COPY smart-contract ./smart-contract
WORKDIR /usr/src/app/smart-contract
RUN cargo build --release --bin agent_network_submit_complete --features livenet

# Final runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

COPY --from=builder /usr/src/app/backend/target/release/backend /usr/local/bin/backend
COPY --from=builder /usr/src/app/smart-contract/target/release/agent_network_submit_complete /usr/local/bin/agent_network_submit_complete

EXPOSE 3000

CMD ["backend"]
