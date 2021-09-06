# Make sure to add any changes to ./Dockerfile.tilt
# as well for the development version of this dockerfile.

FROM golang:1.16

WORKDIR /app
COPY ./sandbox /app
RUN go mod tidy
RUN go build /app/sandbox.go

CMD /app/sandbox
