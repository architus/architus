import os
from sqlalchemy.orm import sessionmaker
from sqlalchemy import create_engine

DB_HOST = 'postgres'
DB_PORT = 5432

try:
    NUM_SHARDS = int(os.environ['NUM_SHARDS'])

    secret_token = os.environ['bot_token']
    db_user = os.environ['db_user']
    db_pass = os.environ['db_pass']
    client_id = os.environ['client_id']
    client_secret = os.environ['client_secret']
    twitter_consumer_key = os.environ['twitter_consumer_key']
    twitter_consumer_secret = os.environ['twitter_consumer_secret']
    twitter_access_token_key = os.environ['twitter_access_token_key']
    twitter_access_token_secret = os.environ['twitter_access_token_secret']
    scraper_token = os.environ['scraper_bot_token']
except KeyError:
    raise EnvironmentError("environment variables not set. Did you create architus.env?") from None

print("creating engine...")
engine = create_engine(f"postgresql://{db_user}:{db_pass}@{DB_HOST}:{DB_PORT}/autbot")


def get_session():
    print("creating a new db session")
    Session = sessionmaker(bind=engine)
    return Session()
