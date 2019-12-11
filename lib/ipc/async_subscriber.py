from uuid import getnode
from os import getpid
from aio_pika import ExchangeType

from lib.ipc.util import poll_for_async_connection


# def on_message(message: IncomingMessage):
#   with message.process():
#       print(" [x] %r:%r" % (message.routing_key, message.body))


class Subscriber:
    def __init__(self):
        self.connection = None
        self.queue = None
        self.id = (getnode() << 15) | getpid()

    async def connect(self, loop):
        self.connection = await poll_for_async_connection(loop)
        channel = await self.connection.channel()
        await channel.set_qos(prefetch_count=1)
        self.event_exchange = await channel.declare_exchange(
            'events', ExchangeType.TOPIC
        )
        self.queue = await channel.declare_queue(
            f'event_queue_{self.id}', exclusive=True
        )

    async def bind_key(self, key):
        await self.queue.bind(self.event_exchange, routing_key=key)

    async def bind_callback(self, callback):
        await self.queue.consume(callback)
