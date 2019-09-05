from aiohttp import web
import socketio

from lib.config import which_shard
from lib.auth import JWT
from lib.async_rpc_client import shardRPC


sio = socketio.AsyncServer(async_mode='aiohttp')
app = web.Application()
sio.attach(app)


shard_client = shardRPC(app.loop)
# @asyncio.coroutine
# def background_task():
#     """Example of how to send server generated events to clients."""
#     ctx = zmq.asyncio.Context()
#     sub = ctx.socket(zmq.SUB)
#     sub.connect('tcp://ipc:6300')
#     sub.setsockopt_string(zmq.SUBSCRIBE, 'event')
#     pub = ctx.socket(zmq.PUB)
#     pub.connect('tcp://ipc:7200')
#     while True:
#         events = json.loads((yield from sub.recv()).decode()[len('event '):])
#         print(f"got {events}")
#         for guild_id, events in events.items():
#             yield from sio.emit('cool_event', events, room=guild_id)


async def index(request):
    return web.Response(
        text='this endpoint is for websockets only, get your http out of here', content_type='text/html')


@sio.event
async def my_event(sid, message):
    await sio.emit('my_response', {'data': message['data']}, room=sid)


@sio.event
async def my_broadcast_event(sid, message):
    await sio.emit('my_response', {'data': message['data']})


@sio.event
async def join(sid, message):
    guild_id = message['room']
    jwt = JWT(token=message['jwt'])
    resp, _ = shard_client.call('is_member', jwt.id, guild_id, routing_key=f"shard_rpc_{which_shard(guild_id)}")
    sio.enter_room(sid, message['room'])
    await sio.emit('my_response', {'data': 'Entered room: ' + message['room']},
                   room=sid)


@sio.event
async def leave(sid, message):
    sio.leave_room(sid, message['room'])
    await sio.emit('my_response', {'data': 'Left room: ' + message['room']},
                   room=sid)


@sio.event
async def close_room(sid, message):
    await sio.emit('my_response',
                   {'data': 'Room ' + message['room'] + ' is closing.'},
                   room=message['room'])
    await sio.close_room(message['room'])


@sio.event
async def my_room_event(sid, message):
    await sio.emit('my_response', {'data': message['data']},
                   room=message['room'])


@sio.event
async def disconnect_request(sid):
    await sio.disconnect(sid)


@sio.event
async def connect(sid, environ):
    print(f"{environ['REMOTE_ADDR']} has connected")
    await sio.emit('my_response', {'data': 'Connected', 'count': 0}, room=sid)


@sio.event
def disconnect(sid):
    print('Client disconnected')


# app.router.add_static('/static', 'static')
app.router.add_get('/', index)


if __name__ == '__main__':
    # sio.start_background_task(background_task)
    web.run_app(app, host='0.0.0.0', port='6000')
