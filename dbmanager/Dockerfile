FROM ubuntu:18.04

RUN apt-get -y update
RUN apt-get install -y postgresql-client
RUN apt-get install -y python3
RUN mkdir /app
RUN mkdir /app/current_migration

VOLUME /app/current_migration

WORKDIR /app
COPY ./dbmanager.py .
COPY ./migrations ./migrations

ENTRYPOINT ["python3", "-u", "dbmanager.py"]
