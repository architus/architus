FROM postgres
RUN sed -i -r 's/#huge_pages.*?/huge_pages = off/g' /usr/share/postgresql/postgresql.conf.sample
WORKDIR /docker-entrypoint-initdb.d
COPY db/*.sql /docker-entrypoint-initdb.d
ENV POSTGRES_USER=autbot
ENV POSTGRES_PASSWORD=autism
ENV POSTGRES_DB=autbot
EXPOSE 5432
