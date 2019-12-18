import json
import asyncio
from functools import wraps

from aiohttp import web
import socketio
from aio_pika import IncomingMessage

from lib.config import which_shard
from lib.auth import JWT
from lib.ipc.async_rpc_client import shardRPC
from lib.ipc.async_subscriber import Subscriber


sio = socketio.AsyncServer(async_mode='aiohttp')
app = web.Application()
sio.attach(app)

loop = asyncio.get_event_loop()
shard_client = shardRPC(loop)
event_subscriber = Subscriber(loop)

auth_nonces = {}


def payload_params(*params):
    '''wraps sio events to make extracting parameters a little easier'''
    def decorator(func):
        @wraps(func)
        async def wrapper(*args, **kwargs):
            # args should look like: (sid, msg), so we're interested in 2nd element
            extracted = {k: v for k, v in args[1]['payload'].items() if k in params}
            kwargs.update(extracted)
            print(kwargs)
            return await func(*args, **kwargs)
        return wrapper
    return decorator


async def event_callback(msg: IncomingMessage):
    '''handles incoming events from the other services'''
    with msg.process():
        body = json.loads(msg.body.decode())

        def register_nonce(nonce: int, jwt: JWT):
            auth_nonces[nonce] = jwt

        if body['method'] == 'register_nonce':
            auth_nonces[body['args']]

        elif body['method'] == 'emit':
            pass
            # sio.emit(body['event'], {'payload': {'message': 'broadcast'}}


@sio.event
async def connect(sid: str, environ: dict):
    print(f"{environ['REMOTE_ADDR']} has connected with sid: {sid}")
    request = environ['aiohttp.request']
    try:
        jwt = JWT(token=request.cookies['token'])
    except KeyError:
        print("No valid token found, logging into unprivileged gateway...")
    else:
        print("Found valid token, logging into elevated gateway...")
        await sio.enter_room(sid, f"{sid}_auth")
        async with sio.session(sid) as session:
            session['token'] = jwt


@sio.event
def disconnect(sid: str):
    print(f'client ({sid}) disconnected')


@sio.event
@payload_params('nonce')
async def request_elevation(sid: str, msg: dict, nonce: int):
    try:
        jwt = JWT(token=auth_nonces[nonce])
    except KeyError:
        print(f"{sid} requested room elevation but didn't provide a valid jwt")
        await sio.emit('elevation_return', {'payload': {'message': "Missing or invalid jwt"}}, room=sid)
    else:
        await sio.enter_room(sid, f"{sid}_auth")
        await sio.emit('elevation_return', {'payload': {'message': "success"}}, room=sid)
        async with sio.session(sid) as session:
            session['token'] = jwt


@sio.event
async def mock_user_event(sid: str, msg: dict):
    args = msg['payload']
    target_shard = which_shard()
    resp, _ = await shard_client.call(
        'interpret',
        **args,
        routing_key=f"shard_rpc_{target_shard}"
    )
    await sio.emit('mock_bot_event', {'payload': resp}, room=sid)


@sio.event
async def spectate(sid: str, msg: dict):
    if f"{sid}_auth" not in sio.rooms(sid):
        return
    # spectate stuff


async def index(request: web.Request):
    return web.Response(text='this endpoint is for websockets only, get your http out of here >:(',
                        content_type='text/html')
app.router.add_get('/', index)

if __name__ == '__main__':
    async def register_clients(shard_client, event_sub):
        await shard_client.connect()
        await (await (await event_sub.connect()).bind_key("gateway_events")).bind_callback(event_callback)

    sio.start_background_task(register_clients, shard_client, event_subscriber)
    web.run_app(app, host='0.0.0.0', port='6000')
