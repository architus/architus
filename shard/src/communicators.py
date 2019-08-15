import asyncio
import zmq
import zmq.asyncio
import json
import os
import time
import functools
from uuid import getnode

from lib.status_codes import StatusCodes


class Comms:

    def __init__(self):
        ctx = zmq.asyncio.Context()

        self.manager_topic = 'm' + str((getnode() << 15) | os.getpid())

        self.sub = ctx.socket(zmq.SUB)
        self.sub.connect(f"tcp://ipc:6200")
        self.msub = ctx.socket(zmq.SUB)
        self.msub.connect(f"tcp://ipc:6200")
        self.pub = ctx.socket(zmq.PUB)
        self.pub.connect(f"tcp://ipc:7300")
        self.pub.setsockopt(zmq.IMMEDIATE, 1)
        self.msub.setsockopt_string(zmq.SUBSCRIBE, str(self.manager_topic))

        self.event_broadcaster = EventBroadcaster(self.pub)

    def register_shard(self):
        # TODO this is garb
        time.sleep(2) # wait for sockets to connect and manager to come up
        print("Requesting shard id...")
        shard_info = asyncio.get_event_loop().run_until_complete(self.manager_request('register'))
        self.shard_id = str(shard_info['shard_id'])
        self.sub.setsockopt_string(zmq.SUBSCRIBE, self.shard_id)
        print(f"Got shard_id {self.shard_id}")
        return shard_info

    @asyncio.coroutine
    def publish(self, topic, data: dict, status_code=StatusCodes.OK_200):
        '''publish a message to the given topic'''
        try:
            data.setdefault('status_code', status_code)
        except AttributeError:
            #TODO
            print("can't setdefault on whatever this is, this should be fixed")
        try:
            data = json.dumps(data)
        except TypeError:
            raise TypeError(f"Couldn't serialize {data}") from None
        yield from self.pub.send(f"{topic} {data}".encode())

    @asyncio.coroutine
    def wait_for_shard_message(self, topic=None):
        '''block until we get a message on the topic'''
        topic = topic if topic is not None else self.shard_id
        msg = (yield from self.sub.recv_string())[len(topic) + 1:]
        print(f"shard {self.shard_id} got msg: {msg}")
        return json.loads(msg)

    @asyncio.coroutine
    def manager_request(self, method, *args):
        yield from self.pub.send_string(f"manager {json.dumps({'method': method, 'topic': self.manager_topic, 'args': args})}")
        return json.loads((yield from self.msub.recv_string())[len(str(self.manager_topic)) + 1:])


class EventBroadcaster:

    def __init__(self, pub):
        self.pub = pub

    @asyncio.coroutine
    def broadcast(self, event, guild_id, data=None):
        data = json.dumps({
            str(guild_id): [{event: data or {}}]
        })
        yield from self.pub.send_string(f"event {data}")

    # broadcast.on_reaction_send(guild)
    def __getattr__(self, name):
        return functools.partial(self.broadcast, name)
