FROM golang:1.15

WORKDIR /app
COPY ./starlark-reactor /app
RUN go build /app/reactor.go

CMD /app/reactor
