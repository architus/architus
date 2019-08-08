FROM python:3.6-alpine

WORKDIR /app
ENV MAGICK_HOME=/usr

RUN apk add build-base musl-dev python3
RUN apk --no-cache --update-cache add gcc python3-dev build-base wget freetype-dev libpng-dev postgresql-dev libffi-dev libxml2-dev libxslt-dev jpeg-dev
RUN apk add  --no-cache ffmpeg
RUN apk add --no-cache imagemagick imagemagick-dev

COPY ./shard/requirements.txt /app


# Install any needed packages specified in requirements.txt
RUN pip3 install --trusted-host pypi.python.org -r requirements.txt
RUN apk del libpng-dev jpeg-dev libffi-dev libxml2-dev python3-dev libxslt-dev freetype-dev


# Copy the current directory contents into the container at /app
COPY  ./shard/ /app/
COPY ./lib /app/lib

# Run app.py when the container launches
CMD ["python3", "-u", "bot.py"]
