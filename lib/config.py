import os
from random import randint
from sqlalchemy.orm import sessionmaker
from sqlalchemy import create_engine
"""
This file loads the environment secrets into memory and also manages database connections
"""

DB_HOST = 'postgres'
DB_PORT = 5432

try:
    NUM_SHARDS = int(os.environ['NUM_SHARDS'])

    secret_token = os.environ['bot_token']
    db_user = os.environ['db_user']
    db_pass = os.environ['db_pass']
    client_id = os.environ['client_id']
    client_secret = os.environ['client_secret']
    domain_name = os.environ['domain_name']
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

print("creating engine...")
engine = create_engine(f"postgresql://{db_user}:{db_pass}@{DB_HOST}:{DB_PORT}/autbot")


def get_session():
    print("creating a new db session")
    Session = sessionmaker(bind=engine)
    return Session()


def which_shard(guild_id=None):
    return randint(0, NUM_SHARDS - 1) if guild_id is None else (int(guild_id) >> 22) % NUM_SHARDS
