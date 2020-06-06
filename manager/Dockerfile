FROM alpine:edge
RUN mkdir -p /var/www
# Set the working directory to /app
WORKDIR /app
COPY ./manager/requirements.txt /app

# Copy the current directory contents into the container at /app
RUN apk update && apk add build-base python3 python3-dev curl gcc linux-headers

RUN curl https://bootstrap.pypa.io/get-pip.py -o get-pip.py
RUN python3 get-pip.py

# Install any needed packages specified in requirements.txt
ENV GRPC_PYTHON_BUILD_EXT_COMPILER_JOBS 16
RUN python3 -m pip install --upgrade pip
RUN python3 -m pip install wheel
RUN python3 -m pip install --trusted-host pypi.python.org -r requirements.txt
RUN apk del build-base python3-dev

COPY ./manager /app/
COPY ./lib /app/lib

ENV NUM_SHARDS=1
CMD ["python3", "-u", "manager_server.py"]
