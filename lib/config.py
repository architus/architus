import os
import zmq
import time
import json
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

connkeeper = {}


def get_session():
    print("creating a new db session")
    Session = sessionmaker(bind=engine)
    return Session()


def get_zmq_socks(id):
    '''used by the api to keep track of connections between requests'''
    if id in connkeeper:
        print(f"reusing old zmq connection for {id}")
        return connkeeper[id]
    else:
        print(f"creating new zmq connection for {id}")
        ctx = zmq.Context()
        sub = ctx.socket(zmq.SUB)
        sub.connect('tcp://ipc:6300')
        sub.setsockopt_string(zmq.SUBSCRIBE, str(id))
        sub.setsockopt(zmq.RCVTIMEO, 60000)
        pub = ctx.socket(zmq.PUB)
        pub.connect('tcp://ipc:7200')
        pub.setsockopt(zmq.IMMEDIATE, 1)

        # make sure our sockets are actually connected before we return them
        print("pinging shard 0...")
        time.sleep(.1)  # garbage race condition, usually this prevents the recv from timing out once
        while True:
            pub.send_string(f"0 {json.dumps({'method': 'ping', 'args': [], 'topic': id, 'id': 0})}")
            try:
                print(sub.recv())
                break
            except zmq.ZMQError as e:
                print(e)
        print("connected")

        connkeeper[id] = (pub, sub)
        return (pub, sub)
