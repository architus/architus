# Create logstash deployment with config
# https://www.elastic.co/guide/en/logstash/current/docker-config.html
FROM docker.elastic.co/logstash/logstash:7.4.0 as logstash
ENV SERVICE=logs/submission-logstash
# Remove the old config & copy custom config
RUN rm -f /usr/share/logstash/pipeline/logstash.conf
COPY $SERVICE/config /usr/share/logstash/config
COPY $SERVICE/pipeline /usr/share/logstash/pipeline
