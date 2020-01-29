FROM ubuntu:18.04

WORKDIR /app
#ENV MAGICK_HOME=/usr

#RUN apt install build-base musl-dev python3
#RUN apt install gcc python3-dev build-base wget freetype-dev libpng-dev postgresql-dev libffi-dev libxml2-dev libxslt-dev jpeg-dev
#RUN apt-get install ffmpeg
#RUN apt-get install imagemagi

RUN apt-get -y update
RUN apt-get -y install python3-pip ffmpeg

COPY ./shard/requirements.txt /app


# Install any needed packages specified in requirements.txt
RUN pip3 install --trusted-host pypi.python.org -r requirements.txt
#RUN apk del libpng-dev jpeg-dev libffi-dev libxml2-dev python3-dev libxslt-dev freetype-dev

ENV PYTHONIOENCODING UTF-8


# Copy the current directory contents into the container at /app
COPY  ./shard/ /app/
COPY ./lib /app/lib

# Run app.py when the container launches
CMD ["python3", "-u", "bot.py"]
