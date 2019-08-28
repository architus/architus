import os
import time
import json
import pika
from sqlalchemy.orm import sessionmaker
from sqlalchemy import create_engine
import uuid

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


def get_connection():
    print("Connecting to rabbitmq...")
    credentials = pika.PlainCredentials('hello', 'hello')
    parameters = pika.ConnectionParameters('rabbit', 5672, '/', credentials)
    while True:
        try:
            return pika.BlockingConnection(parameters)
        except pika.exceptions.AMQPConnectionError as e:
            print(f"rabbit doesn't seem to be up, trying again in 1 {e}")
            time.sleep(1)


print("connected to rabbit")


def get_client(id):
    try:
        return connkeeper[id]
    except KeyError:
        connkeeper[id] = shardRPC()
        return connkeeper[id]


class shardRPC:
    def __init__(self):
        self.connection = get_connection()
        self.channel = self.connection.channel()
        result = self.channel.queue_declare(queue='', exclusive=True)
        self.callback_queue = result.method.queue
        self.channel.basic_consume(
            queue=self.callback_queue,
            on_message_callback=self.on_response,
            auto_ack=True)

    def on_response(self, ch, method, props, body):
        if self.corr_id == props.correlation_id:
            resp = json.loads(body)
            self.resp = resp['resp']
            self.status_code = resp['sc']

    def call(self, method, *args, routing_key=None, **kwargs):
        assert routing_key is not None
        self.resp = None
        self.corr_id = str(uuid.uuid4())
        print("sending:")
        print(json.dumps({'method': method, 'args': args, 'kwargs': kwargs}))
        self.channel.basic_publish(
            exchange='',
            routing_key='rpc_queue',
            properties=pika.BasicProperties(
                reply_to=self.callback_queue,
                correlation_id=self.corr_id,
            ),
            body=json.dumps({'method': method, 'args': args, 'kwargs': kwargs}))
        while self.resp is None:
            self.connection.process_data_events()
        return self.resp, self.status_code
