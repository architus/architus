from aiohttp import web
import asyncio
import socketio
from functools import partial

from lib.config import which_shard
from lib.auth import JWT
from lib.async_rpc_client import shardRPC


sio = socketio.AsyncServer(async_mode='aiohttp')
app = web.Application()
sio.attach(app)

shard_client = shardRPC(asyncio.get_event_loop())

async def index(request):
    return web.Response(
        text='this endpoint is for websockets only, get your http out of here >:(',
        content_type='text/html'
    )

app.router.add_get('/', index)

@sio.event
async def connect(sid, environ):
    print(f"{environ['REMOTE_ADDR']} has connected with sid: {sid}")

@sio.event
def disconnect(sid):
    print('client ({sid}) disconnected')

@sio.event
async def request_elevation(sid, msg):
    try:
        token = msg['payload']['jwt']
        jwt = JWT(token=token)
    except (jwt.exceptions.InvalidTokenError, KeyError):
        print(f"{sid} requested room elevation but didn't provide a valid jwt")
        sio.emit('elevation_return' {'payload': {'message': "Missing or invalid jwt"}}, room=sid)
    else:
        sio.enter_room(sid, f"{sid}_auth")
        sio.emit('elevation_return' {'payload': {'message': "success"}}, room=sid)

@sio.event
async def mock_user_event(sid, msg):
    args = msg['payload']
    target_shard = which_shard()
    resp, _ = await shard_client.call(
        'interpret',
        **args,
        routing_key=f"shard_rpc_{target_shard}"
    )
    await sio.emit('mock_bot_event', {'payload': resp}, room=sid)

@sio.event
async def spectate(sid, msg):
    if f"{sid}_auth" not in sio.rooms(sid):
        return
    # spectate stuff

if __name__ == '__main__':
    sio.start_background_task(shard_client.connect)
    web.run_app(app, host='0.0.0.0', port='6000')
