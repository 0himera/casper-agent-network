# Build stage using bookworm to match runtime stage glibc
FROM rust:slim-bookworm AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/app

# Copy projects
COPY backend ./backend
COPY smart-contract ./smart-contract

# Build backend
WORKDIR /usr/src/app/backend
RUN cargo build --release

# Build smart contract CLI submit_complete
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
