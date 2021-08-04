FROM golang:1.16

WORKDIR /app
COPY ./sandbox /app
RUN go mod tidy
RUN go build /app/sandbox.go

CMD /app/sandbox
