from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
import yaml
# from src.commands import *
# import src.commands as command_modules

secret_token = None
db_user = None
db_pass = None
sessions = {}

DB_HOST = '127.0.0.1'
DB_PORT = 5432

try:
    with open('.secrets.yaml') as f:
        data = yaml.safe_load(f)

    secret_token = data['bot_token']
    db_user = data['db_user']
    db_pass = data['db_pass']
    client_id = data['client_id']
    client_secret = data['client_secret']
    twitter_consumer_key = data['twitter_consumer_key']
    twitter_consumer_secret = data['twitter_consumer_secret']
    twitter_access_token_key = data['twitter_access_token_key']
    twitter_access_token_secret = data['twitter_access_token_secret']
    scraper_token = data['scraper_bot_token']

except Exception as e:
    print(e)
    print('error reading .secrets.yaml, make it you aut')


def get_session():
    engine = create_engine(f"postgresql://{db_user}:{db_pass}@{DB_HOST}:{DB_PORT}/autbot")
    Session = sessionmaker(bind=engine)
    return Session()


session = get_session()
