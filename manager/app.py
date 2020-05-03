import asyncio
import os
from datetime import datetime, timedelta
import base64

from lib.ipc import async_rpc_server
from lib.config import logger, domain_name
from lib.hoar_frost import HoarFrostGenerator


class Manager:
    """Manages shards.  assigns shard nodes their ids and checks if they are alive"""
    def __init__(self, total_shards):
        logger.info(f"Number of shards: {total_shards}")
        self.hoarfrost_gen = HoarFrostGenerator()
        self.total_shards = total_shards
        self.registered = [False for _ in range(total_shards)]
        self.last_checkin = {}
        self.store = {}

    async def handle_task(self, method, *args, **kwargs):
        try:
            return (await (getattr(self, method)(*args, **kwargs)), 200)
        except Exception as e:
            logger.exception(f"caught: '{e}' while executing '{method}'")
            return {'message': f"caught: '{e}' while executing '{method}'"}

    async def health_check(self):
        while True:
            await asyncio.sleep(1)
            for shard, last_checkin in self.last_checkin.items():
                if last_checkin is not None and last_checkin < datetime.now() - timedelta(seconds=5):
                    logger.error(f"--- SHARD {shard} MISSED ITS HEARTBEAT, DEREGISTERING... ---")
                    self.registered[shard] = False
                    self.last_checkin[shard] = None

    async def register(self):
        """Returns the next shard id that needs to be filled as well as the total shards"""
        if all(self.registered):
            raise Exception("Shard trying to register even though we're full")
        i = next(i for i in range(self.total_shards) if not self.registered[i])
        logger.info(f'Shard requested id, assigning {i + 1}/{self.total_shards}...')
        self.store[i] = {'shard_id': i}
        self.registered[i] = True
        return {'shard_id': i, 'shard_count': self.total_shards}

    async def all_guilds(self):
        """Return information about all guilds that the bot is in, including their admins"""
        guilds = []
        for shard, shard_store in self.store.items():
            guilds += shard_store.get('guilds', ())
        return guilds

    async def guild_count(self):
        """Return guild and user count information"""
        guild_count = 0
        user_count = 0
        for shard, shard_store in self.store.items():
            guilds = shard_store.get('guilds', ())
            guild_count += len(guilds)
            for guild in guilds:
                user_count += guild['member_count']
        return {'guild_count': guild_count, 'user_count': user_count}

    async def guild_update(self, shard_id, guilds):
        """Update the manager with the latest information about a shard's guilds"""
        logger.debug(f"someone sent guild list containing {len(guilds)} guilds")
        self.store[int(shard_id)]['guilds'] = guilds
        return {"message": "thanks"}

    async def checkin(self, shard_id):
        self.last_checkin[shard_id] = datetime.now()
        self.registered[shard_id] = True
        return "nice"

    async def publish_file(self, location: str = 'assets', name: str = '', filetype: str = 'png', data: str = ''):
        assert data != ''
        if name == '':
            name = str(self.hoarfrost_gen.generate())
        directory = f'/var/www/{location}'

        if not os.path.exists(directory):
            os.makedirs(directory)
        with open(f'{directory}/{name}.{filetype}', 'wb') as f:
            logger.info(f'writing {directory}/{name}.{filetype}')
            f.write(base64.b64decode(data))
        return {'url': f'https://cdn.{domain_name}/{location}/{name}.{filetype}'}


loop = asyncio.get_event_loop()
manager = Manager(int(os.environ['NUM_SHARDS']))

loop.create_task(manager.health_check())
loop.create_task(
    async_rpc_server.start_server(
        loop,
        'manager_rpc',
        manager.handle_task
    )
)
loop.run_forever()
