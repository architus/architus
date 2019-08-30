from functools import partial
from aio_pika import connect, Message
import json
import asyncio


async def on_message(entry_point, exchange, message):
    with message.process():
        msg = json.loads(message.body.decode())
        print(f"remote call of '{msg['method']}' with {len(msg['args'])} args and {len(msg['kwargs'])} kwargs")

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
    while True:
        try:
            rabbit_connection = await connect("amqp://hello:hello@rabbit/", loop=loop)
            break
        except Exception as e:
            print(f"Waiting to connect to rabbit: {e}")
            await asyncio.sleep(1)
    print("connected")

    channel = await rabbit_connection.channel()
    queue = await channel.declare_queue(listener_queue)

    await queue.consume(
        partial(
            on_message,
            entry_point,
            channel.default_exchange
        )
    )
