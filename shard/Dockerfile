FROM ubuntu:18.04

WORKDIR /app
#ENV MAGICK_HOME=/usr
ENV DEBIAN_FRONTEND=noninteractive
ENV TZ=America/New_York

#RUN apt install build-base musl-dev python3
#RUN apt install gcc python3-dev build-base wget freetype-dev libpng-dev postgresql-dev libffi-dev libxml2-dev libxslt-dev jpeg-dev
#RUN apt-get install ffmpeg
#RUN apt-get install imagemagi

RUN apt-get -y update
RUN apt install -y software-properties-common
RUN add-apt-repository ppa:deadsnakes/ppa
RUN apt-get -y update
RUN apt-get -y install python3.8-dev python3-pip ffmpeg pkg-config git
RUN apt-get -y install texlive texlive-latex-extra dvipng
RUN apt-get -y install libxml2-dev libxslt-dev python-libxslt1 libpq-dev libffi-dev

COPY ./shard/requirements.txt /app

RUN git clone -b master https://github.com/architus/discord.py
WORKDIR ./discord.py
RUN python3.8 -m pip install -U .[voice]
WORKDIR /app


# Install any needed packages specified in requirements.txt
RUN python3.8 -m pip install --trusted-host pypi.python.org -r requirements.txt
#RUN apk del libpng-dev jpeg-dev libffi-dev libxml2-dev python3-dev libxslt-dev freetype-dev

ENV PYTHONIOENCODING UTF-8


# Copy the current directory contents into the container at /app
COPY  ./shard/ /app/
COPY ./lib /app/lib

# Run app.py when the container launches
CMD ["python3.8", "-u", "bot.py"]
