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
ENV RUSTC_BOOTSTRAP=1

WORKDIR /usr/src/app

# Copy Cargo manifests for workspace dependency caching
COPY Cargo.toml ./Cargo.toml
COPY agentnet-core/Cargo.toml ./agentnet-core/Cargo.toml
COPY validator-engine/Cargo.toml ./validator-engine/Cargo.toml
COPY validator-node/Cargo.toml ./validator-node/Cargo.toml
COPY backend/Cargo.toml ./backend/Cargo.toml
COPY smart-contract/Cargo.toml smart-contract/Cargo.lock* ./smart-contract/

# Create stub source files for pre-building dependencies
RUN mkdir -p agentnet-core/src validator-engine/src validator-node/src backend/src smart-contract/src smart-contract/bin && \
    echo "pub fn dummy() {}" > agentnet-core/src/lib.rs && \
    echo "pub fn dummy() {}" > validator-engine/src/lib.rs && \
    echo "fn main() {}" > validator-node/src/main.rs && \
    echo "fn main() {}" > backend/src/main.rs && \
    echo "pub fn dummy() {}" > smart-contract/src/lib.rs && \
    echo "fn main() {}" > smart-contract/bin/submit_complete.rs && \
    echo "fn main() {}" > smart-contract/bin/submit_result.rs && \
    echo "fn main() {}" > smart-contract/bin/submit_validation.rs && \
    echo "fn main() {}" > smart-contract/bin/finalize_task.rs && \
    echo "fn main() {}" > smart-contract/bin/decay_reputation.rs && \
    echo "fn main() {}" > smart-contract/bin/build_contract.rs && \
    echo "fn main() {}" > smart-contract/bin/build_schema.rs && \
    echo "fn main() {}" > smart-contract/bin/cli.rs && \
    echo "fn main() {}" > smart-contract/bin/livenet_deploy.rs && \
    echo "fn main() {}" > smart-contract/bin/register_agent.rs && \
    echo "fn main() {}" > smart-contract/bin/update_metadata.rs

# Pre-build smart contract CLI dependencies
WORKDIR /usr/src/app/smart-contract
RUN cargo build --release --bin agent_network_submit_complete --bin agent_network_submit_result --bin agent_network_submit_validation --bin agent_network_finalize_task --bin agent_network_decay_reputation --features livenet

# Pre-build backend dependencies
WORKDIR /usr/src/app
RUN cargo build --release -p backend

# Copy real source code
COPY agentnet-core/src ./agentnet-core/src
COPY validator-engine/src ./validator-engine/src
COPY validator-engine/prompts ./validator-engine/prompts
COPY backend/src ./backend/src
COPY smart-contract/src ./smart-contract/src
COPY smart-contract/bin ./smart-contract/bin

# Force Cargo to rebuild actual binaries
RUN touch agentnet-core/src/lib.rs validator-engine/src/lib.rs backend/src/main.rs smart-contract/src/lib.rs smart-contract/bin/*.rs

# Build actual backend binary
WORKDIR /usr/src/app
RUN cargo build --release -p backend

# Build actual smart contract CLI binaries
WORKDIR /usr/src/app/smart-contract
RUN cargo build --release --bin agent_network_submit_complete --bin agent_network_submit_result --bin agent_network_submit_validation --bin agent_network_finalize_task --bin agent_network_decay_reputation --features livenet

# Final runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    curl \
    && rm -rf /var/lib/apt/lists/*

RUN groupadd -g 10001 appuser && \
    useradd -r -u 10001 -g appuser appuser

WORKDIR /app

COPY --from=builder /usr/src/app/target/release/backend /usr/local/bin/backend
COPY --from=builder /usr/src/app/smart-contract/target/release/agent_network_submit_complete /usr/local/bin/agent_network_submit_complete
COPY --from=builder /usr/src/app/smart-contract/target/release/agent_network_submit_result /usr/local/bin/agent_network_submit_result
COPY --from=builder /usr/src/app/smart-contract/target/release/agent_network_submit_validation /usr/local/bin/agent_network_submit_validation
COPY --from=builder /usr/src/app/smart-contract/target/release/agent_network_finalize_task /usr/local/bin/agent_network_finalize_task
COPY --from=builder /usr/src/app/smart-contract/target/release/agent_network_decay_reputation /usr/local/bin/agent_network_decay_reputation

RUN chmod +x /usr/local/bin/*

USER appuser

EXPOSE 3000

CMD ["backend"]
