# Build protobuf stubs in Ruby
# https://developers.google.com/protocol-buffers/docs/reference/ruby-generated
FROM ubuntu:20.04 as builder
ENV REPO_ROOT=/opt/architus
ENV PROTO=lib/ipc/proto
# Install the protobuf compiler
RUN apt-get update -q \
    && apt-get install -y -q "protobuf-compiler"
# Copy protobuf definitions
WORKDIR $REPO_ROOT/$PROTO
COPY $PROTO/logs/event.proto logs/event.proto
# Compile the protobufs
RUN mkdir -p /opt/proto
RUN protoc --proto_path=$REPO_ROOT/$PROTO --ruby_out=/opt/proto logs/event.proto

# Create logstash deployment with config
# https://www.elastic.co/guide/en/logstash/current/docker-config.html
FROM docker.elastic.co/logstash/logstash:7.4.0 as logstash
ENV SERVICE=logs/submission-logstash
# Install the protobuf codec plugin
RUN logstash-plugin install logstash-codec-protobuf
# Remove the old config & copy custom config
RUN rm -f /usr/share/logstash/pipeline/logstash.conf
COPY $SERVICE/config /usr/share/logstash/config
COPY $SERVICE/pipeline /usr/share/logstash/pipeline
# Copy compiled ruby files
COPY --from=builder /opt/proto/logs/event_pb.rb /opt/logstash/proto/logs/event_pb.rb
