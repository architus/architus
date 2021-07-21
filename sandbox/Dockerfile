FROM golang:1.16

WORKDIR /app
COPY ./sandbox /app
RUN go build /app/sandbox.go

# Uncomment this line for a dev build
ENV HTTP_PROXY=""
# Uncomment this line for a prod build
# ENV HTTP_PROXY="http://archit.us:1234"

CMD /app/sandbox
