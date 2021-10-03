ARG REPO_ROOT=/opt/architus
ARG SERVICE=logs/gateway-ingress
ARG GATEWAY_QUEUE_LIB=logs/gateway-queue-lib
ARG AMQP_POOL_LIB=lib/amqp-pool-rs
ARG CONFIG_BACKOFF_LIB=lib/config-backoff-rs
ARG PROTO=lib/ipc/proto

# Build service
FROM rust:1.52 as builder
RUN rustup component add rustfmt
ARG REPO_ROOT
ARG SERVICE
ARG GATEWAY_QUEUE_LIB
ARG LOGS_LIB
ARG AMQP_POOL_LIB
ARG CONFIG_BACKOFF_LIB
ARG PROTO
# Copy logs/gateway-ingress service itself
WORKDIR $REPO_ROOT/$SERVICE
COPY $SERVICE/src src
COPY $SERVICE/Cargo.lock .
COPY $SERVICE/Cargo.toml .
COPY $SERVICE/build.rs .
# Copy logs/gateway-gateway-lib protos
WORKDIR $REPO_ROOT/$GATEWAY_QUEUE_LIB
COPY $GATEWAY_QUEUE_LIB/proto proto
# Copy shared amqp pool library
WORKDIR $REPO_ROOT/$AMQP_POOL_LIB
COPY $AMQP_POOL_LIB/src src
COPY $AMQP_POOL_LIB/Cargo.toml .
# Copy shared config backoff library
WORKDIR $REPO_ROOT/$CONFIG_BACKOFF_LIB
COPY $CONFIG_BACKOFF_LIB/src src
COPY $CONFIG_BACKOFF_LIB/Cargo.toml .
# Copy protobuf definitions
WORKDIR $REPO_ROOT/$PROTO
COPY $PROTO/feature-gate.proto .
# Build via Cargo
WORKDIR $REPO_ROOT/$SERVICE
RUN cargo build --release
RUN cp $REPO_ROOT/$SERVICE/target/release/logs-gateway-ingress /opt/logs-gateway-ingress

# Create minimal deployment
FROM debian:buster-slim as deployment
ARG SERVICE
ENV DEBIAN_FRONTEND=noninteractive
RUN apt-get update -q \
    && apt-get install -y -q "libssl1.1" "tini" "ca-certificates" \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/logs-gateway-ingress /usr/bin/logs-gateway-ingress
COPY $SERVICE/config.default.toml /etc/architus/config.toml
ENV RUST_BACKTRACE=1
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/bin/logs-gateway-ingress"]
CMD ["/etc/architus/config.toml"]
