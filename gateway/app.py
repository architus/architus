import json
import asyncio
from functools import wraps

from aiohttp import web
import socketio
from aio_pika import IncomingMessage
from jwt.exceptions import InvalidTokenError

from lib.config import which_shard
from lib.auth import JWT
from lib.ipc.async_rpc_client import shardRPC
from lib.ipc.async_subscriber import Subscriber
from lib.ipc.async_rpc_server import start_server
from lib.status_codes import StatusCodes as sc


sio = socketio.AsyncServer(
    async_mode='aiohttp',
    cors_allowed_origins='*'  # ('https://*.archit.us:443', 'https://archit.us:443', 'http://localhost:3000')
)
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


async def register_nonce(method, *args, **kwargs):
    if method == 'register_nonce':
        try:
            auth_nonces[args[0]] = args[1]
        except IndexError:
            pass
        else:
            return {'message': 'registered'}, sc.OK_200
    elif method == 'demote_connection':
        # TODO
        return {'message': 'demoted :)'}, sc.OK_200
    return {'message': 'invaild arguments'}, sc.BAD_REQUEST_400


async def event_callback(msg: IncomingMessage):
    '''handles incoming events from the other services'''
    with msg.process():
        body = json.loads(msg.body.decode())
        guild_id = body['guild_id']
        # private = body['private']
        await sio.emit('log_pool', body, room=f'guild_{guild_id}')
        print(f"emitting action ({body['action_number']})")

        # sio.emit(body['event'], {'payload': {'message': 'broadcast'}}


@sio.event
async def connect(sid: str, environ: dict):
    print(f"{environ['REMOTE_ADDR']} has connected with sid: {sid}")
    request = environ['aiohttp.request']
    try:
        jwt = JWT(token=request.cookies['token'])
    except (InvalidTokenError, KeyError):
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
        del auth_nonces[nonce]
    except (InvalidTokenError, KeyError):
        print(f"{sid} requested room elevation but didn't provide a valid jwt")
        await sio.emit('elevation_return', {'payload': {'message': "Missing or invalid jwt"}}, room=sid)
    else:
        await sio.enter_room(sid, f"{sid}_auth")
        await sio.emit('elevation_return', {'payload': {'message': "success"}}, room=sid)
        async with sio.session(sid) as session:
            session['token'] = jwt


@sio.event
async def mock_user_event(sid: str, kwargs: dict):
    resp, _ = await shard_client.interpret(**kwargs, routing_key=f"shard_rpc_{which_shard()}")
    await sio.emit('mock_bot_event', resp, room=sid)


@sio.event
async def spectate(sid: str, guild_id: int):
    if f"{sid}_auth" not in sio.rooms(sid):
        return
    async with sio.session(sid) as session:
        member, _ = shard_client.is_member(session['token'].id, guild_id)
        if member:
            sio.enter_room(sid, f"guild_{guild_id}")


async def index(request: web.Request):
    return web.Response(text='this endpoint is for websockets only, get your http out of here >:(',
                        content_type='text/html')
app.router.add_get('/', index)

if __name__ == '__main__':
    async def register_clients(shard_client, event_sub):
        await shard_client.connect()
        await (await (await event_sub.connect()).bind_key("gateway.*")).bind_callback(event_callback)

    sio.start_background_task(register_clients, shard_client, event_subscriber)
    sio.start_background_task(start_server, loop, 'gateway_rpc', register_nonce)
    web.run_app(app, host='0.0.0.0', port='6000')
