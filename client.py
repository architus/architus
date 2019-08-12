import asyncio
import time
import socketio

loop = asyncio.get_event_loop()
sio = socketio.AsyncClient()
start_timer = None


@sio.event
async def connect():
    print("connected to server...\njoining room '607606128985112596'...")
    await sio.emit('join', {'room': '607606128985112596'})
    #await send_ping()
    #await asyncio.sleep(10)
    #await sio.emit('leave', {'room': '123456789'})

@sio.event
async def cool_event(data):
    print("I got an event")
    print(data)

@sio.event
async def pong_from_server(data):
    global start_timer
    latency = time.time() - start_timer
    print('latency is {0:.2f} ms'.format(latency * 1000))
    await sio.sleep(1)
    await send_ping()


async def start_server():
    print('hello I\'m a UI :)')
    await sio.connect('http://127.0.0.1:6000')
    await sio.wait()


if __name__ == '__main__':
    loop.run_until_complete(start_server())
