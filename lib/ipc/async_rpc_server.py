from functools import partial
from aio_pika import Message
import json

from lib.ipc.util import poll_for_async_connection
from lib.config import logger


async def on_message(entry_point, exchange, message):
    with message.process():
        msg = json.loads(message.body.decode())
        logger.debug(f"remote call of '{msg['method']}' with {len(msg['args'])} args and {len(msg['kwargs'])} kwargs")

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
