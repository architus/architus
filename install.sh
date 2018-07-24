#!/bin/bash
sudo apt-get install ffmpeg

declare -a requirements=(
    aiohttp
    discord
    matplotlib
    pytz
    psycopg2-binary
    sqlalchemy
    pathlib
    youtube_dl
    pafy
    spotipy
    mutagen
    beautifulsoup4
    bs4
    unicode-slugify
    titlecase
    logzero
    lyricwikia
    PyYAML
    lxml
)

python3 -m venv .venv
source "./.venv/bin/activate"
for i in "${requirements[@]}"
do
    pip install "$i"
done
deactivate
