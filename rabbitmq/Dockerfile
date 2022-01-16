FROM rabbitmq:3.9.7-management-alpine

ENV RABBITMQ_USER hello
ENV RABBITMQ_PASSWORD hello

COPY rabbitmq/init.sh /init.sh
EXPOSE 15672

# Define default command
CMD ["/init.sh"]
