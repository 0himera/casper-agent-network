# Build stage using bookworm to match runtime stage glibc
FROM rust:slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Limit Cargo parallelism to prevent excessive memory usage/OOM during compilation
ENV CARGO_BUILD_JOBS=4

# Stub local workspace libraries to cache dependencies
# 1. validator-engine stub
RUN mkdir -p backend/validator/src && echo "pub fn dummy() {}" > backend/validator/src/lib.rs
COPY backend/validator/Cargo.toml ./backend/validator/

# 2. backend stub
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs
COPY backend/Cargo.toml ./backend/

# 3. smart-contract stub
RUN mkdir -p smart-contract/src && echo "pub mod agent_network {}" > smart-contract/src/lib.rs
RUN mkdir -p smart-contract/bin && echo "fn main() {}" > smart-contract/bin/submit_complete.rs
COPY smart-contract/Cargo.toml smart-contract/rust-toolchain* ./smart-contract/

# Pre-build dependencies
WORKDIR /usr/src/app/backend
RUN cargo build --release

WORKDIR /usr/src/app/smart-contract
RUN cargo build --release --bin agent_network_submit_complete --features livenet

# Copy real source code
WORKDIR /usr/src/app
COPY backend/src ./backend/src
COPY backend/validator ./backend/validator
COPY smart-contract/src ./smart-contract/src
COPY smart-contract/bin ./smart-contract/bin

# Force Cargo to rebuild the stubbed targets since host files have older timestamps
RUN touch backend/src/main.rs \
          backend/validator/src/lib.rs \
          smart-contract/src/lib.rs \
          smart-contract/bin/submit_complete.rs

# Build actual binaries
WORKDIR /usr/src/app/backend
RUN cargo build --release

WORKDIR /usr/src/app/smart-contract
RUN cargo build --release --bin agent_network_submit_complete --features livenet

# Final runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

# Copy compiled binaries
COPY --from=builder /usr/src/app/backend/target/release/backend /usr/local/bin/backend
COPY --from=builder /usr/src/app/smart-contract/target/release/agent_network_submit_complete /usr/local/bin/agent_network_submit_complete

EXPOSE 3000

CMD ["backend"]
