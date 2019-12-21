import json
from aio_pika import Message, DeliveryMode, ExchangeType

from lib.ipc.util import poll_for_async_connection


class Emitter:
    def __init__(self, loop):
        self.loop = loop
        self.connection = None
        self.event_exchange = None

    async def connect(self):
        # Perform connection
        self.connection = await poll_for_async_connection(self.loop)

        channel = await self.connection.channel()

        self.event_exchange = await channel.declare_exchange(
            'events', ExchangeType.TOPIC
        )

        return self

    async def close(self):
        await self.connection.close()

    async def emit(self, routing_key, body):

        message = Message(
            json.dumps(body).encode(),
            delivery_mode=DeliveryMode.PERSISTENT
        )

        # Sending the message
        await self.event_exchange.publish(
            message, routing_key=routing_key
        )
