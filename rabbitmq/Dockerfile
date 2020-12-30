FROM rabbitmq:3.8.9-management-alpine

ENV RABBITMQ_USER hello
ENV RABBITMQ_PASSWORD hello

ADD rabbitmq/init.sh /init.sh
EXPOSE 15672

# Define default command
CMD ["/init.sh"]
