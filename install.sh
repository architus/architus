#!/bin/bash
RED='\033[0;31m'
NC='\033[0m'
sudo apt-get install ffmpeg || echo -e "$RED Please install ffmpeg with your package manager $NC"
echo "You'll need to compile imagemagick with webp support manually on Ubuntu 16!!!!"
echo "https://askubuntu.com/questions/251950/imagemagick-convert-cant-convert-to-webp"

declare -a requirements=(
    aiohttp
    aiofiles
    discord
    matplotlib
    pytz
    pillow
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
)

python3.6 -m venv .venv
source "./.venv/bin/activate"
for i in "${requirements[@]}"
do
    pip install "$i"
done
deactivate
