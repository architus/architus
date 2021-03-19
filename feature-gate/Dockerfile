# Build service
FROM rust:1.50 as builder
RUN rustup component add rustfmt
ENV REPO_ROOT=/opt/architus
ENV SERVICE=feature-gate
ENV PROTO=lib/ipc/proto
# Copy feature-gate service itself
WORKDIR $REPO_ROOT/$SERVICE
COPY $SERVICE/src src
COPY $SERVICE/Cargo.lock .
COPY $SERVICE/Cargo.toml .
COPY $SERVICE/build.rs .
# Copy protobuf definitions
WORKDIR $REPO_ROOT/$PROTO
COPY $PROTO/feature-gate.proto .
# Build via Cargo
WORKDIR $REPO_ROOT/$SERVICE
RUN cargo build --release
RUN cp $REPO_ROOT/$SERVICE/target/release/feature-gate /opt/feature-gate

# Create minimal deployment
FROM debian:buster-slim as deployment
ARG DEBIAN_FRONTEND=noninteractive
RUN apt-get update -q \
    && apt-get install -y -q "libssl1.1" "tini" "libpq-dev" \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /opt/feature-gate /usr/bin/feature-gate
ENV RUST_BACKTRACE=1
ENTRYPOINT ["/usr/bin/tini", "--", "/usr/bin/feature-gate"]
