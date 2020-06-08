# Debian variant
FROM postgres
ENV POSTGRES_USER=autbot
ENV POSTGRES_PASSWORD=autism
ENV POSTGRES_DB=autbot
EXPOSE 5432
RUN apt-get update && DEBIAN_FRONTEND=noninteractive apt-get install -y python3

# Configure the root working directory & copy migrations
RUN mkdir /db
RUN mkdir /db/current_migration && chown postgres /db/current_migration
RUN mkdir /db/migrations
VOLUME /db/current_migration
COPY ./db/bootstrap.py /db
COPY ./db/migrations /db/migrations

# Run migration-running bootstrap.py file
ENTRYPOINT [ "python3", "-u", "/db/bootstrap.py" ]
