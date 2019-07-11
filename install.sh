#!/bin/bash
RED='\033[0;31m'
NC='\033[0m'
echo "Installing ffmpeg... the packaged version on Ubuntu is out of date and you may need to update this manually"
sudo apt-get install ffmpeg || echo -e "$RED Please install ffmpeg with your package manager $NC"
echo "You'll need to compile imagemagick with webp support manually on Ubuntu 16!!!!"
echo "https://askubuntu.com/questions/251950/imagemagick-convert-cant-convert-to-webp"

python3.6 -m venv .venv
source "./.venv/bin/activate"
pip install -r requirements.txt
deactivate
