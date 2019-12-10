FROM postgres
WORKDIR /docker-entrypoint-initdb.d
COPY db/*.sql /docker-entrypoint-initdb.d
ENV POSTGRES_USER=autbot
ENV POSTGRES_PASSWORD=autism
ENV POSTGRES_DB=autbot
EXPOSE 5432
