import asyncio
import json
import zmq
import zmq.asyncio
import os
from datetime import datetime, timedelta


@asyncio.coroutine
def listen_for_stuff(loop):
    ctx = zmq.asyncio.Context()
    sub = ctx.socket(zmq.SUB)
    sub.connect('tcp://ipc:6300')
    sub.setsockopt_string(zmq.SUBSCRIBE, 'manager')
    # sub.setsockopt(zmq.RCVTIMEO, 2000)
    pub = ctx.socket(zmq.PUB)
    pub.connect('tcp://ipc:7200')
    manager = Manager(int(os.environ['NUM_SHARDS']))
    while True:
        try:
            task = (yield from sub.recv_string())[len('manager '):]
            print(f"manager got task {task}")
        except Exception as e:
            print(f"Malformed ipc request or something: {e}")
            continue
        loop.create_task(manager.handle_task(pub, json.loads(task)))


class Manager:
    def __init__(self, total_shards):
        print(f"Number of shards: {total_shards}")
        self.total_shards = total_shards
        self.registered = 0
        self.last_checkin = {}
        self.store = {}

    @asyncio.coroutine
    def handle_task(self, pub, task):
        print("future ensured")
        resp = yield from getattr(self, task['method'])(task['topic'], *task['args'])
        yield from pub.send_string(f"{task['topic']} {json.dumps(resp)}")

    async def register(self, topic):
        if self.registered >= self.total_shards:
            raise Exception("Shard trying to register even though we're full")
        self.registered += 1
        print(f'Shard requested id, assigning {self.registered}/{self.total_shards}...')
        self.store[topic] = {'shard_id': self.registered - 1}
        return {'shard_id': self.registered - 1, 'shard_count': self.total_shards}

    async def guild_count(self, topic):
        guild_count = 0
        user_count = 0
        for shard, shard_store in self.store.items():
            guilds = shard_store.get('guilds', ())
            guild_count += len(guilds)
            for guild in guilds:
                user_count += guild['member_count']
        return {'guild_count': guild_count, 'user_count': user_count}

    async def guild_update(self, topic, guilds):
        '''shards only method'''
        print(f"{topic} sent some guilds: {guilds}")
        self.store[topic]['guilds'] = guilds
        return {"message": "thanks"}

    async def checkin(self, shard_id):
        '''shards only method'''
        self.last_checkin[shard_id] = datetime.now()
        return "nice"

    def is_everyone_still_alive(self):
        for shard, checkin in self.last_checkin.items():
            if checkin > datetime.now() - timedelta(seconds=30):
                print(f"Shard {shard} is DOWN!")
                return False
        return True


loop = asyncio.get_event_loop()
loop.run_until_complete(listen_for_stuff(loop))
loop.close()
