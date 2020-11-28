# syntax=docker/dockerfile:experimental

# Build service
FROM rust:1.48 as builder
RUN rustup component add rustfmt
ENV REPO_ROOT=/opt/architus
ENV SERVICE=logs/logging
ENV LOGS_LIB=lib/id-rs
ENV PROTO=lib/ipc/proto
# Copy logging service itself
WORKDIR $REPO_ROOT/$SERVICE
COPY $SERVICE/src src
COPY $SERVICE/Cargo.lock .
COPY $SERVICE/Cargo.toml .
COPY $SERVICE/build.rs .
# Copy shared id library
WORKDIR $REPO_ROOT/$LOGS_LIB
COPY $LOGS_LIB/src src
COPY $LOGS_LIB/Cargo.lock .
COPY $LOGS_LIB/Cargo.toml .
# Copy protobuf definitions
WORKDIR $REPO_ROOT/$PROTO
COPY $PROTO/logging.proto .
# Build via Cargo
WORKDIR $REPO_ROOT/$SERVICE
RUN --mount=type=cache,target=/usr/local/cargo/registry \
	--mount=type=cache,target=$REPO_ROOT/$SERVICE/target \
	--mount=type=cache,target=$REPO_ROOT/$LOGS_LIB/target \
	cargo build --release
RUN cp $REPO_ROOT/$SERVICE/target/release/logging-service /opt/logging-service

# Create minimal deployment
FROM debian:buster-slim as deployment
COPY --from=builder /opt/logging-service /usr/bin/logging-service
ENTRYPOINT ["/usr/bin/logging-service"]
