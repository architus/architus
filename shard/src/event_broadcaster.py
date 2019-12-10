import asyncio
import functools
import json


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
