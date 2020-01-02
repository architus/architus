import asyncio
import sys
import time
import socketio

loop = asyncio.get_event_loop()
sio = socketio.AsyncClient()
start_timer = None

guild_id = 607637793107345431

@sio.event
async def connect():
    print("connected to server...")
    nonce = 1923512129
    if len(sys.argv) > 1:
        print("requesting elevated gateway...")
        await sio.emit('request_elevation', int(sys.argv[1]))

    print(f"requesting spectate guild {guild_id}")
    await sio.emit('spectate', guild_id)


@sio.event
async def elevation_return(*data):
    print(f'elevation_return: {data}')


@sio.event
async def log_pool(*data):
    print(f'log_pool: {data}')


async def start_server():
    print('hello I\'m a UI :)')
    # await sio.connect('https://gateway.develop.archit.us')
    await sio.connect('http://127.0.0.1:6000')
    await sio.wait()


if __name__ == '__main__':
    loop.run_until_complete(start_server())
