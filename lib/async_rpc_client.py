import uuid
import json
from aio_pika import connect, IncomingMessage, Message


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
        self.connection = await connect(
            "amqp://hello:hello@rabbit/", loop=self.loop
        )
        self.channel = await self.connection.channel()
        self.callback_queue = await self.channel.declare_queue(
            exclusive=True
        )
        await self.callback_queue.consume(self.on_response)

        return self

    def on_response(self, message: IncomingMessage):
        future = self.futures.pop(message.correlation_id)
        resp = json.loads(message.body)
        future.set_result((resp['resp'], resp['sc']))

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

        return await future
