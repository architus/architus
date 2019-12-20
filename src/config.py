from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
# from src.commands import *
# import src.commands as command_modules

secret_token = None
db_user = None
db_pass = None
sessions = {}

try:
    lines = [line.rstrip('\n') for line in open('.secret_token')]
    secret_token = lines[0]
    db_user = lines[1]
    db_pass = lines[2]
    client_id = lines[3]
    client_secret = lines[4]
    alphavantage_api_key = lines[5]
    twitter_consumer_key = lines[6]
    twitter_consumer_secret = lines[7]
    twitter_access_token_key = lines[8]
    twitter_access_token_secret = lines[9]
    scraper_token = lines[10]

except Exception as e:
    print(e)
    print('error reading .secret_token, make it you aut')


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
