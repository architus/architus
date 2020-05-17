import os
from random import randint
from sqlalchemy.orm import sessionmaker
from sqlalchemy import create_engine
from datetime import datetime, timezone
from contextlib import suppress
import re
import logging

with suppress(ImportError):
    import asyncpg

"""
This file loads the environment secrets into memory and also manages database connections
"""

logger = logging.getLogger("architus")
logger.setLevel(logging.DEBUG)
handler = logging.StreamHandler()
handler.setFormatter(logging.Formatter('%(asctime)s:%(levelname)s:%(name)s:%(module)s: %(message)s'))
logger.addHandler(handler)

DB_HOST = 'postgres'
DB_PORT = 5432

DISCORD_EPOCH = datetime(2015, 1, 1, tzinfo=timezone.utc)

try:
    NUM_SHARDS = int(os.environ['NUM_SHARDS'])

    secret_token = os.environ['bot_token']
    db_user = os.environ['db_user']
    db_pass = os.environ['db_pass']
    client_id = os.environ['client_id']
    client_secret = os.environ['client_secret']
    domain_name = os.environ['domain_name']
    is_prod = os.environ['domain_name'] == 'archit.us'
    alphavantage_api_key = os.environ['alphavantage_api_key']
    twitter_consumer_key = os.environ['twitter_consumer_key']
    twitter_consumer_secret = os.environ['twitter_consumer_secret']
    twitter_access_token_key = os.environ['twitter_access_token_key']
    twitter_access_token_secret = os.environ['twitter_access_token_secret']
    scraper_token = os.environ['scraper_bot_token']
except KeyError:
    raise EnvironmentError("environment variables not set. Did you create architus.env?") from None

API_ENDPOINT = 'https://discordapp.com/api/v6'
REDIRECT_URI = f'https://api.{domain_name}/redirect'

logger.debug("creating db engine...")
try:
    engine = create_engine(f"postgresql://{db_user}:{db_pass}@{DB_HOST}:{DB_PORT}/autbot")
except Exception as e:
    logger.warn(f"Couldn't create engine, maybe you don't care {e}")


def get_session():
    logger.debug("creating a new db session")
    Session = sessionmaker(bind=engine)
    return Session()


class AsyncConnWrapper:
    def __init__(self):
        self.conn = None

    async def connect(self):
        self.conn = await asyncpg.connect(f"postgresql://{db_user}:{db_pass}@{DB_HOST}:{DB_PORT}/autbot")


def which_shard(guild_id=None):
    return randint(0, NUM_SHARDS - 1) if guild_id is None else (int(guild_id) >> 22) % NUM_SHARDS


allowed_origins_regexes = [
    re.compile(o) for o in (
        r'https:\/\/archit\.us\/app',
        r'https:\/\/.*\.archit\.us\/app',
        r'http:\/\/localhost:3000\/app',
        r'https:\/\/[-A-Za-z0-9]{24}--architus\.netlify\.com\/app',
        r'https:\/\/deploy-preview-[0-9]+--architus\.netlify\.com\/app',
    )
]
