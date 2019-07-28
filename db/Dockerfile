FROM postgres
WORKDIR /docker-entrypoint-initdb.d
COPY *.sql /docker-entrypoint-initdb.d
ENV POSTGRES_USER=autbot
ENV POSTGRES_PASSWORD=example
ENV POSTGRES_DB=autbot
EXPOSE 5432
