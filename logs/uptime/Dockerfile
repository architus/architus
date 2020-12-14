# syntax=docker/dockerfile:experimental

# Build service
FROM rust:1.48 as builder
RUN rustup component add rustfmt
ENV REPO_ROOT=/opt/architus
ENV SERVICE=logs/uptime
ENV PROTO=lib/ipc/proto
# Copy logs/uptime service itself
WORKDIR $REPO_ROOT/$SERVICE
COPY $SERVICE/src src
COPY $SERVICE/Cargo.lock .
COPY $SERVICE/Cargo.toml .
COPY $SERVICE/build.rs .
# Copy protobuf definitions
WORKDIR $REPO_ROOT/$PROTO
COPY $PROTO/logs/uptime.proto logs/uptime.proto
# Build via Cargo
WORKDIR $REPO_ROOT/$SERVICE
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=$REPO_ROOT/$SERVICE/target \
	cargo build --release
RUN cp $REPO_ROOT/$SERVICE/target/release/logs-uptime /opt/logs-uptime

# Create minimal deployment
FROM debian:buster-slim as deployment
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update -q \
    && apt-get install -y -q "libssl1.1" "tini" \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/logs-uptime /usr/bin/logs-uptime
ENV RUST_BACKTRACE=1
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/bin/logs-uptime"]
