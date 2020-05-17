import json
import asyncio

from aiohttp import web
import socketio
from aio_pika import IncomingMessage
from jwt.exceptions import InvalidTokenError

from lib.config import which_shard, logger
from lib.auth import JWT  # , gateway_authenticated as authenticated
from lib.ipc.async_rpc_client import shardRPC
from lib.ipc.async_subscriber import Subscriber
from lib.ipc.async_rpc_server import start_server
from lib.status_codes import StatusCodes as s
from lib.discord_requests import async_list_guilds_request
from lib.pool_types import PoolType


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


class Counter:
    def __init__(self):
        self._count = -2

    @property
    def count(self):
        self._count += 2
        return self._count


async def register_nonce(method, *args, **kwargs):
    if method == 'register_nonce':
        try:
            auth_nonces[args[0]] = args[1]
        except IndexError:
            pass
        else:
            return {'message': 'registered'}, s.OK_200
    elif method == 'demote_connection':
        # TODO
        return {'message': 'demoted :)'}, s.OK_200
    return {'message': 'invaild arguments'}, s.BAD_REQUEST_400


async def event_callback(msg: IncomingMessage):
    '''handles incoming events from the other services'''
    with msg.process():
        body = json.loads(msg.body.decode())
        guild_id = body['guild_id']
        # private = body['private']
        await sio.emit('log_pool', body, room=f'guild_{guild_id}')
        # print(f"emitting action ({body['action_number']})")

        # sio.emit(body['event'], {'payload': {'message': 'broadcast'}}


class CustomNamespace(socketio.AsyncNamespace):

    def __init__(self, *args, **kwargs):
        self.counter = Counter()

        super().__init__(*args, **kwargs)

    async def on_connect(self, sid: str, environ: dict):
        logger.debug(f"{environ['REMOTE_ADDR']} has connected with sid: {sid}")
        request = environ['aiohttp.request']
        try:
            jwt = JWT(token=request.cookies['token'])
        except (InvalidTokenError, KeyError):
            logger.info("No valid token found, logging into unprivileged gateway...")
        else:
            logger.info("Found valid token, logging into elevated gateway...")
            sio.enter_room(sid, f"{sid}_auth")
            async with sio.session(sid) as session:
                session['jwt'] = jwt

    async def on_pool_all_request(self, sid: str, data):
        async with self.session(sid) as session:
            _jwt = session['jwt']
        _id = data['_id']
        guild_id = data.get('guild_id', None)
        type = data['type']
        if type == PoolType.GUILD:
            resp, sc = await async_list_guilds_request(_jwt)
            if sc == s.OK_200:
                resp, sc = await shard_client.tag_autbot_guilds(resp, _jwt.id, routing_key=f"shard_rpc_{which_shard()}")
                if sc == s.OK_200:
                    await sio.emit(
                        'pool_all_request_response',
                        _id=_id,
                        data=resp,
                        finished=True,
                        room=f"{sid}_auth"
                    )
                    return

        else:
            resp, sc = await shard_client.pool_all_request(
                guild_id, type, routing_key=f"shard_rpc_{which_shard(guild_id)}")
            if sc == s.OK_200:
                await sio.emit(
                    'pool_all_request_response',
                    _id=_id,
                    data=resp['data'],
                    finished=True,
                    room=f"{sid}_auth"
                )
                return
        await sio.emit('error', room=sid)

    def on_disconnect(self, sid: str):
        logger.debug(f'client ({sid}) disconnected')

    async def on_request_elevation(self, sid: str, nonce: int):
        logger.debug(f"{sid} requesting elevation...")
        try:
            jwt = JWT(token=auth_nonces[nonce])
            del auth_nonces[nonce]
        except (InvalidTokenError, KeyError):
            logger.info(f"{sid} requested room elevation but didn't provide a valid jwt")
            await sio.emit('elevation_return', {'message': "Missing or invalid jwt"}, room=sid)
        else:
            logger.debug(f"valid nonce provided, granting access...")
            sio.enter_room(sid, f"{sid}_auth")
            await sio.emit('elevation_return', {'message': "success"}, room=sid)
            async with sio.session(sid) as session:
                session['token'] = jwt

    async def on_mock_user_event(self, sid: str, kwargs: dict):
        guild_id = kwargs['guildId']
        resp, _ = await shard_client.handle_mock_user_action(**kwargs, routing_key=f"shard_rpc_{which_shard(guild_id)}")
        await sio.emit('mock_bot_event', resp, room=sid)

    async def on_spectate(self, sid: str, guild_id: int):
        if f"{sid}_auth" not in sio.rooms(sid):
            return
        async with sio.session(sid) as session:
            member, _ = await shard_client.is_member(
                session['token'].id,
                guild_id,
                routing_key=f"shard_rpc_{which_shard(guild_id)}"
            )
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
    sio.register_namespace(CustomNamespace('/'))
    web.run_app(app, host='0.0.0.0', port='6000')
