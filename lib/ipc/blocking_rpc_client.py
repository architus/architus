import time
import json
import pika
import uuid
from functools import partial

from lib.ipc.util import poll_for_connection

connkeeper = {}


def get_rpc_client(id):
    """manager function to keep track of connections between processes in wsgi

    :param id: unique id to collect client
    :returns: shardRPC -- rpc client object
    """
    try:
        return connkeeper[id]
    except KeyError:
        connkeeper[id] = shardRPC()
        return connkeeper[id]


class shardRPC:
    """Client to handle rabbit response ids and queues and stuff"""
    def __init__(self):
        self.connection = poll_for_connection()
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

    def __getattr__(self, name):
        return partial(self.call, name)

    def call(self, method, *args, routing_key=None, **kwargs):
        """Remotely call a method

        :param method: name of method to call
        :param *args: arguments to pass to method
        :param routing_key: queue to route to in rabbitmq
        :param **kwargs: keyword args to pass to method
        """
        assert routing_key is not None
        print(f'calling {method} on queue: {routing_key}')
        self.resp = None
        self.corr_id = str(uuid.uuid4())
        self.channel.basic_publish(
            exchange='',
            routing_key=routing_key,
            properties=pika.BasicProperties(
                reply_to=self.callback_queue,
                correlation_id=self.corr_id,
            ),
            body=json.dumps({'method': method, 'args': args, 'kwargs': kwargs}))
        while self.resp is None:
            self.connection.process_data_events()
        return self.resp, self.status_code
