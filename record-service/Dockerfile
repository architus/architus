FROM rust:1.43

WORKDIR /app
COPY ./record-service/ /app
COPY ./lib/ipc/proto/manager.proto /app/proto
RUN apt-get -y update
RUN apt-get -y install libclang-7-dev automake autoconf libtool libopus-dev libssl-dev
RUN apt-get -y install zip
RUN rustup component add rustfmt
RUN cargo install --path .

CMD ["mic", "7777"]
