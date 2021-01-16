FROM golang:1.15

WORKDIR /app
COPY ./sandbox /app
RUN go build /app/sandbox.go

CMD /app/sandbox
