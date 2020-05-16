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
RUN apt-get -y install python3.8-dev python3-pip ffmpeg pkg-config cron git
RUN apt-get -y install texlive texlive-latex-extra dvipng
RUN apt-get -y install libxml2-dev libxslt-dev python-libxslt1 libpq-dev libffi-dev

# Install architus re2 library
RUN git clone https://github.com/architus/re2
WORKDIR ./re2
RUN make install

# Install pyre2 python library
WORKDIR /app
RUN git clone https://github.com/facebook/pyre2
WORKDIR ./pyre2
RUN python3.8 setup.py build
RUN python3.8 setup.py install
RUN python3.8 setup.py check

WORKDIR /app

COPY ./shard/requirements.txt /app

# Install any needed packages specified in requirements.txt
RUN python3.8 -m pip install --trusted-host pypi.python.org -r requirements.txt
#RUN apk del libpng-dev jpeg-dev libffi-dev libxml2-dev python3-dev libxslt-dev freetype-dev

RUN mkdir -p /etc/cron.d
RUN touch /var/log/cron.log
RUN echo "@hourly root python3.8 -m pip install youtube-dl --upgrade > /var/log/cron.log 2>&1\n# Empty line for syntax" >> /etc/cron.d/youtube-dl_update
RUN chmod 0744 /etc/cron.d/youtube-dl_update
RUN crontab /etc/cron.d/youtube-dl_update
RUN cron

ENV PYTHONIOENCODING UTF-8


# Copy the current directory contents into the container at /app
COPY  ./shard/ /app/
COPY ./lib /app/lib

# Run app.py when the container launches
ENV LD_LIBRARY_PATH="/usr/local/lib:${LD_LIBRARY_PATH}"
CMD cron && pyhton3.8 -u bot.py
ENV LD_LIBRARY_PATH="/usr/local/lib:${LD_LIBRARY_PATH}"
CMD ["python3.8", "-u", "bot.py"]