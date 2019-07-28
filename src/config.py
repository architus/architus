from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
import yaml
# from src.commands import *
# import src.commands as command_modules

secret_token = None
db_user = None
db_pass = None
sessions = {}

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
    scraper_token = data['sraper_bot_token']

except Exception as e:
    print(e)
    print('error reading .secrets.yaml, make it you aut')


def get_session(pid=None):
    if pid in sessions:
        return sessions[pid]
    print("creating postgres session")
    try:
        engine = create_engine("postgresql://{}:{}@localhost/autbot".format(db_user, db_pass))
        Session = sessionmaker(bind=engine)
        session = Session()
        sessions[pid] = session

    except Exception as e:
        session = None
        print('failed to connect to database')
        print(e)
    return session


session = get_session()
