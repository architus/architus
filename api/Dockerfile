FROM alpine:3.10

# Set the working directory to /app
WORKDIR /app
COPY ./api/requirements.txt /app

# Copy the current directory contents into the container at /app
RUN apk update && apk add linux-headers pcre-dev uwsgi-python3
RUN apk add build-base musl-dev python3 #zeromq-dev libzmq
RUN apk add --virtual build-deps gcc python3-dev musl-dev postgresql-dev
RUN apk add --no-cache --virtual .pynacl_deps libffi-dev


# Install any needed packages specified in requirements.txt
RUN pip3 install --trusted-host pypi.python.org -r requirements.txt
RUN apk del build-base musl-dev python3-dev

COPY ./api/ /app/
COPY ./lib/python-common /app/lib

EXPOSE 5000

ENV NUM_SHARDS=1


# Run app.py when the container launches
CMD ["uwsgi", "--ini", "config_uwsgi.ini"]
