#!/bin/bash
sudo apt-get install ffmpeg
sudo apt-get install libenchant1c2a
echo "You'll need to compile imagemagick with webp support manually!!!!"
echo "https://askubuntu.com/questions/251950/imagemagick-convert-cant-convert-to-webp"

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
    emoji
    wand
    pyenchant
)

python3 -m venv .venv
source "./.venv/bin/activate"
for i in "${requirements[@]}"
do
    pip install "$i"
done
deactivate
