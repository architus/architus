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
    await sio.emit('mock_user_event', {'guildId': 19, 'content': '!set testing::pwned', 'messageId': 2, 'allowedCommands': ('set', 'remove'), 'action': 3001, 'silent': False})

    await sio.emit('mock_user_event', {'guildId': 19, 'content': '!schedule event 12pm', 'messageId': 4, 'allowedCommands': ('schedule', 'poll'), 'action': 3001, 'silent': False})
    # nonce = 1923512129
    if len(sys.argv) > 1:
        print("requesting elevated gateway...")
        await sio.emit('free_elevation', {'token': sys.argv[1]})


    print(f"requesting spectate guild {guild_id}")
    await sio.emit('spectate', guild_id)
    #print(f"requesting spectate guild {guild_id}")
    #await sio.emit('spectate', guild_id)

    #for i in range(10):
    #    await sio.emit('pool_all_request', {'type': 'guild', 'guildId': guild_id, '_id': 1})
   # await sio.emit('pool_all_request', {'type': 'member', 'guildId': guild_id, '_id': 2})
    #await sio.emit('pool_all_request', {'type': 'role', 'guildId': guild_id, '_id': 3})
    #await sio.emit('pool_all_request', {'type': 'autoResponse', 'guildId': guild_id, '_id': 5})
    #await sio.emit('pool_request', {'type': 'member', 'guildId': guild_id, 'ids': ['214037134477230080', '1111111111111'], '_id': 7})
    #await sio.emit('pool_request', {'type': 'user', 'guildId': None, 'ids': ['214037134477230080', '109462069484486656'], '_id': 9})
    await sio.emit('pool_request', {'type': 'customEmoji', 'guildId': guild_id, 'ids': ['711543547145097377'], '_id': 9})
    await sio.emit('pool_all_request', {'type': 'customEmoji', 'guildId': guild_id, '_id': 11})

    # await th


@sio.event
async def elevation_return(*data):
    print(f'elevation_return: {data}')

@sio.event
async def pool_response(*args, **kwargs):
    print(f"{args}")

@sio.event
async def error(*args, **kwargs):
    print("error")
    print(args)

@sio.event
async def log_pool(*data):
    print(f'log_pool: {data}')


@sio.event
async def mock_bot_event(*data):
    print(f'mock_bot_event: {data}')


async def start_server():
    print('hello I\'m a UI :)')
    await sio.connect('https://gateway.develop.archit.us')
    #await sio.connect('http://gateway.local.archit.us:6000')
    await sio.wait()


if __name__ == '__main__':
    loop.run_until_complete(start_server())
