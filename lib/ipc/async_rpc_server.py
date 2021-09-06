from functools import partial
from aio_pika import Message
from asyncio import sleep
import json

from lib.ipc.util import poll_for_async_connection
from lib.config import logger


async def on_message(entry_point, exchange, message):
    with message.process():
        msg = json.loads(message.body.decode())

        ret, status_code = await entry_point(msg['method'], *msg['args'], **msg['kwargs'])

        response = json.dumps({
            'sc': int(status_code),
            'resp': ret,
        }).encode()

        await exchange.publish(
            Message(
                body=response,
                correlation_id=message.correlation_id
            ),
            routing_key=message.reply_to
        )


async def start_server(loop, listener_queue, entry_point):
    rabbit_connection = await poll_for_async_connection(loop)

    channel = await rabbit_connection.channel()
    queue = await channel.declare_queue(listener_queue)

    await queue.consume(
        partial(
            on_message,
            entry_point,
            channel.default_exchange
        )
    )

    while True:
        await sleep(60)
        if rabbit_connection.heartbeat_last < loop.time() - 60:
            logger.warning("seems as though we aren't connected to rabbit anymore :thinking:")
            await start_server(loop, listener_queue, entry_point)
