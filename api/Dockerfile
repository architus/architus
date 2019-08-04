# Use an official Python runtime as a parent image
FROM alpine:edge

# Set the working directory to /app
WORKDIR /app
COPY requirements.txt /app

# Copy the current directory contents into the container at /app
RUN apk update && apk add linux-headers pcre-dev uwsgi-python3
RUN apk update && apk add build-base libzmq musl-dev python3 zeromq-dev
RUN apk update && \
    apk add --virtual build-deps gcc python3-dev musl-dev && \
    apk add postgresql-dev
#RUN apk update && apk add py3-zmq


# Install any needed packages specified in requirements.txt
RUN pip3 install --trusted-host pypi.python.org -r requirements.txt
RUN apk del build-base musl-dev python3-dev zeromq-dev

COPY . /app

EXPOSE 5000

ENV NUM_SHARDS=1


# Run app.py when the container launches
CMD ["uwsgi", "--ini", "config_uwsgi.ini"]
