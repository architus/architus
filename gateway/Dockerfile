FROM alpine:3.10

# Set the working directory to /app
WORKDIR /app
COPY ./gateway/requirements.txt /app

# Copy the current directory contents into the container at /app
RUN apk update && apk add python3 linux-headers pcre-dev uwsgi-python3 build-base
# RUN apk add build-base libzmq musl-dev python3 zeromq-dev
RUN apk add --virtual build-deps gcc python3-dev musl-dev postgresql-dev


# Install any needed packages specified in requirements.txt
RUN pip3 install --trusted-host pypi.python.org -r requirements.txt
# RUN apk del build-base musl-dev python3-dev zeromq-dev
RUN apk del python3-dev

COPY ./gateway/ /app/
COPY ./lib/python-common /app/lib

EXPOSE 6000

# Run app.py when the container launches
CMD ["python3", "-u", "app.py"]
