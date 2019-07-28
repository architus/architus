# Use an official Python runtime as a parent image
FROM alpine:edge

# Set the working directory to /app
WORKDIR /app

# Copy the current directory contents into the container at /app
COPY . /app
RUN apk update && apk add build-base libzmq musl-dev python3 zeromq-dev
RUN apk update && \
    apk add --virtual build-deps gcc python3-dev musl-dev && \
    apk add postgresql-dev
#RUN apk update && apk add py3-zmq


# Install any needed packages specified in requirements.txt
RUN pip3 install --trusted-host pypi.python.org -r requirements.txt

EXPOSE 5000
RUN apk del build-base musl-dev python3-dev zeromq-dev


# Run app.py when the container launches
CMD ["python3", "app.py"]
