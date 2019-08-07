import yaml
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker
import zmq
import os
import time
import json

DB_HOST = 'postgres'
DB_PORT = 5432
NUM_SHARDS = int(os.environ['NUM_SHARDS'])

try:
    with open('.secrets.yaml') as f:
        data = yaml.safe_load(f)
except FileNotFoundError:
    with open('../.secrets.yaml') as f:
        data = yaml.safe_load(f)

client_id = data['client_id']
client_secret = data['client_secret']
db_user = data['db_user']
db_pass = data['db_pass']

print("creating engine and connkeeper")
engine = create_engine(f"postgresql://{db_user}:{db_pass}@{DB_HOST}:{DB_PORT}/autbot")
connkeeper = {}


def get_session():
    print("creating a new db session")
    Session = sessionmaker(bind=engine)
    return Session()


def get_pubsub(id):
    if id in connkeeper:
        print(f"reusing old zmq connection for {id}")
        return connkeeper[id]
    else:
        print(f"creating new zmq connection for {id}")
        ctx = zmq.Context()
        sub = ctx.socket(zmq.SUB)
        sub.connect('tcp://ipc:6300')
        sub.setsockopt_string(zmq.SUBSCRIBE, str(id))
        sub.setsockopt(zmq.RCVTIMEO, 2000)
        pub = ctx.socket(zmq.PUB)
        pub.connect('tcp://ipc:7200')

        # make sure our sockets are actually connected before we return them
        print("pinging shard 0...")
        time.sleep(.1)  # garbage race condition, usually this prevents the recv from timing out once
        while True:
            pub.send_string(f"0 {json.dumps({'method': 'ping', 'args': [], 'topic': id})}")
            try:
                print(sub.recv())
                break
            except zmq.ZMQError as e:
                print(e)
        print("connected")

        connkeeper[id] = (pub, sub)
        return (pub, sub)
