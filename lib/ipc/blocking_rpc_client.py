import json
import uuid
import time
from functools import partial
from threading import Thread, Lock

import pika

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
        self.hb_lock = Lock()
        result = self.channel.queue_declare(queue='', exclusive=True)
        self.callback_queue = result.method.queue
        self.channel.basic_consume(
            queue=self.callback_queue,
            on_message_callback=self.on_response,
            auto_ack=True)

        hb_thread = Thread(target=self.heartbeat, daemon=True)
        hb_thread.start()

    def heartbeat(self):
        '''
        heartbeat to the rabbit server.

        All rabbit connections require heartbeat, but flask doesn't have time to do this itself.
        `process_data_events` is not thread safe, however
        '''
        while True:
            with self.hb_lock:
                self.connection.process_data_events(time_limit=0)
            time.sleep(30)

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
        with self.hb_lock:
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
