#!/bin/bash

declare -a requirements=(
    aiohttp
    discord
    matplotlib
    pytz
    psycopg2-binary
    sqlalchemy
)

python3 -m venv .venv
source "./.venv/bin/activate"
for i in "${requirements[@]}"
do
    pip install -r requirements.txt
done
deactivate
