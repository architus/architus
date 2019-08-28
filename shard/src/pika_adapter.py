from functools import partial
from aio_pika import connect, Message
import json
import asyncio


async def on_message(api, exchange, message):
    with message.process():
        msg = json.loads(message.body.decode())
        print(f"remote call of '{msg['method']}' with {len(msg['args'])} args and {len(msg['kwargs'])} kwargs")

        ret, status_code = await api.api_entry(msg['method'], *msg['args'], **msg['kwargs'])

        response = json.dumps({
            'sc': int(status_code),
            'resp': ret,
        }).encode()

        print(message.reply_to)

        await exchange.publish(
            Message(
                body=response,
                correlation_id=message.correlation_id
            ),
            routing_key=message.reply_to
        )
        print('Request complete')


async def main(bot):
    while True:
        try:
            rabbit_connection = await connect("amqp://hello:hello@rabbit/", loop=bot.loop)
            break
        except Exception as e:
            print(f"catching this garb {e}")
            await asyncio.sleep(1)

    channel = await rabbit_connection.channel()
    queue = await channel.declare_queue('rpc_queue')

    api = bot.get_cog('Api')

    await queue.consume(
        partial(
            on_message,
            api,
            channel.default_exchange
        )
    )
