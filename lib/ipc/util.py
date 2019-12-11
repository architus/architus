import socket
import asyncio
import time

from contextlib import suppress

with suppress(ModuleNotFoundError):
    from aio_pika import connect
with suppress(ModuleNotFoundError):
    import pika


async def poll_for_async_connection(loop):
    name = socket.gethostname()
    while True:
        try:
            return await connect("amqp://hello:hello@rabbit/", loop=loop)
        except ConnectionError as e:
            print(f"{name} is waiting to connect to rabbit: {e}")
            await asyncio.sleep(3)
        finally:
            print(f"{name} successfully connected to rabbit")


def poll_for_connection():
    name = socket.gethostname()
    credentials = pika.PlainCredentials('hello', 'hello')
    # TODO heartbeat should really, really not be disabled this is very bad
    parameters = pika.ConnectionParameters('rabbit', 5672, '/', credentials, heartbeat=0)
    while True:
        try:
            return pika.BlockingConnection(parameters)
        except pika.exceptions.AMQPConnectionError as e:
            # except Exception as e:
            print(f"{name} is waiting to connect to rabbit: {e}")
            time.sleep(3)
        finally:
            print(f"{name} successfully connected to rabbit")
