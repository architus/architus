import uuid
import json
from functools import partial
from aio_pika import IncomingMessage, Message

from lib.ipc.util import poll_for_async_connection


class shardRPC:
    """Client to handle rabbit response ids and queues and stuff"""
    def __init__(self, loop, default_key=None):
        self.default_key = default_key
        self.connection = None
        self.channel = None
        self.callback_queue = None
        self.futures = {}
        self.loop = loop

    async def connect(self):
        self.connection = await poll_for_async_connection(self.loop)
        self.channel = await self.connection.channel()
        self.callback_queue = await self.channel.declare_queue(
            exclusive=True
        )
        await self.callback_queue.consume(self.on_response)

        return self

    def on_response(self, message: IncomingMessage):
        with message.process():
            future = self.futures.pop(message.correlation_id)
            resp = json.loads(message.body)
            future.set_result((resp['resp'], resp['sc']))

    def __getattr__(self, name):
        return partial(self.call, name)

    async def call(self, method, *args, routing_key=None, **kwargs):
        """Remotely call a method

        :param method: name of method to call
        :param *args: arguments to pass to method
        :param routing_key: queue to route to in rabbitmq
        :param **kwargs: keyword args to pass to method
        """
        routing_key = self.default_key if routing_key is None else routing_key
        correlation_id = str(uuid.uuid4())
        future = self.loop.create_future()

        self.futures[correlation_id] = future
        if self.channel is None:
            await self.connect()

        await self.channel.default_exchange.publish(
            Message(
                json.dumps(
                    {
                        'method': method,
                        'args': args,
                        'kwargs': kwargs,
                    }
                ).encode(),
                content_type='text/plain',
                correlation_id=correlation_id,
                reply_to=self.callback_queue.name,
            ),
            routing_key=routing_key,
        )
        try:
            return await asyncio.wait_for(future, timeout=5)
        except asyncio.TimeoutError as e:
            logger.exception('')
            return e, 500
        return await future
