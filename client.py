import asyncio
import time
import socketio

loop = asyncio.get_event_loop()
sio = socketio.AsyncClient()
start_timer = None


@sio.event
async def connect():
    print("connected to server...\njoining room '607606128985112596'...")
    await sio.emit('join', {'room': '607606128985112596', 'jwt': 'eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJhY2Nlc3NfdG9rZW4iOiJldmw4QmxMTWVtSjVrbHFHWFRZM01sRTNiQ2tiSFYiLCJleHBpcmVzX2luIjo2MDQ4MDAsInJlZnJlc2hfdG9rZW4iOiIyOU5vcjNmWWpsUFUyTEZ0R0pwZk04eHdJOVd3akciLCJ1c2VybmFtZSI6ImpvaG55YnVyZCIsImRpc2NyaW1pbmF0b3IiOiIxMDIyIiwiYXZhdGFyIjoiOWNlNWEwMDY1NDhmMWFlM2U1YzhmYjkxYTRkNjc3ZTQiLCJpZCI6IjIxNDAzNzEzNDQ3NzIzMDA4MCJ9.yuAbfPFavctPy5R3lauPN6BLM-D__557SksLLjC3bzc'})

    print("'interpret', {'content': '!schedule event 12pm', 'guild_id': 1234, 'message_id': 1, 'allowed_commands': ['schedule']})")
    await sio.emit('interpret', {'content': '!schedule event 12pm', 'guild_id': 1234, 'message_id': 1, 'allowed_commands': ['schedule']})

@sio.event
async def cool_event(data):
    print("I got an event")
    print(data)

@sio.event
async def my_response(data):
    print(f'recv: {data}')

@sio.event
async def pong_from_server(data):
    global start_timer
    latency = time.time() - start_timer
    print('latency is {0:.2f} ms'.format(latency * 1000))
    await sio.sleep(1)
    await send_ping()


async def start_server():
    print('hello I\'m a UI :)')
    await sio.connect('https://ws.archit.us')
    await sio.wait()


if __name__ == '__main__':
    loop.run_until_complete(start_server())
