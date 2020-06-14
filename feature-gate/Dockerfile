FROM rust:1.44

WORKDIR /app
COPY ./feature-gate/ /app
RUN apt update -y
RUN apt install -y musl-dev libpq-dev

ENV DATABASE_URL=postgresql://autbot:autism@postgres:5432/autbot
RUN cargo build
CMD /app/target/debug/feature-gate
