import socket
import asyncio
import time

from contextlib import suppress

from lib.config import logger

with suppress(ModuleNotFoundError):
    from aio_pika import connect
with suppress(ModuleNotFoundError):
    import pika


async def poll_for_async_connection(loop):
    name = socket.gethostname()
    while True:
        try:
            return await connect("amqp://hello:hello@rabbit/", loop=loop)
        except (ConnectionError, Exception) as e:
            logger.debug(f"{name} is waiting to connect to rabbit: {e}")
            await asyncio.sleep(3)
        finally:
            logger.debug(f"{name} successfully connected to rabbit")


def poll_for_connection():
    name = socket.gethostname()
    credentials = pika.PlainCredentials('hello', 'hello')
    parameters = pika.ConnectionParameters('rabbit', 5672, '/', credentials, heartbeat=200)
    while True:
        try:
            return pika.BlockingConnection(parameters)
        except pika.exceptions.AMQPConnectionError as e:
            # except Exception as e:
            logger.debug(f"{name} is waiting to connect to rabbit: {e}")
            time.sleep(3)
        finally:
            logger.debug(f"{name} successfully connected to rabbit")
