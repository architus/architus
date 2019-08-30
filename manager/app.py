import asyncio
import os
from datetime import datetime

from lib import async_rpc_server


class Manager:
    def __init__(self, total_shards):
        print(f"Number of shards: {total_shards}")
        self.total_shards = total_shards
        self.registered = 0
        self.last_checkin = {}
        self.store = {}

    async def handle_task(self, method, *args, **kwargs):
        return (await (getattr(self, method)(*args, **kwargs)), 200)

    async def register(self):
        if self.registered >= self.total_shards:
            raise Exception("Shard trying to register even though we're full")
        self.registered += 1
        print(f'Shard requested id, assigning {self.registered}/{self.total_shards}...')
        self.store[self.registered - 1] = {'shard_id': self.registered - 1}
        return {'shard_id': self.registered - 1, 'shard_count': self.total_shards}

    async def all_guilds(self):
        guilds = []
        for shard, shard_store in self.store.items():
            guilds += shard_store.get('guilds', ())
        return guilds

    async def guild_count(self):
        guild_count = 0
        user_count = 0
        for shard, shard_store in self.store.items():
            guilds = shard_store.get('guilds', ())
            guild_count += len(guilds)
            for guild in guilds:
                user_count += guild['member_count']
        return {'guild_count': guild_count, 'user_count': user_count}

    async def guild_update(self, shard_id, guilds):
        '''shards only method'''
        print("someone sent guild list containing {len(guilds)} guilds")
        self.store[int(shard_id)]['guilds'] = guilds
        return {"message": "thanks"}

    async def checkin(self, shard_id):
        '''shards only method'''
        self.last_checkin[shard_id] = datetime.now()
        return "nice"


loop = asyncio.get_event_loop()
manager = Manager(int(os.environ['NUM_SHARDS']))

loop.create_task(
    async_rpc_server.start_server(
        loop,
        'manager_rpc',
        manager.handle_task
    )
)
loop.run_forever()
