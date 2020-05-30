FROM rust:1.43

WORKDIR /app
COPY ./record-service/ /app
RUN apt-get -y update
RUN apt-get -y install libclang-7-dev automake autoconf libtool
RUN cargo install --path .

CMD ["mic", "7777"]
