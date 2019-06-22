import json
import traceback
import asyncio
from discord.ext.commands import Cog, Context

class MockMember(object):
    def __init__(self):
        self.id = 0

class MockChannel(object):
    def __init__(self, sends):
        self.sends = sends
    async def send(self, *args):
        for thing in args:
            self.sends.append(thing)
        return MockMessage(self.sends)

class MockGuild(object):
    def __init__(self):
        self.region = 'us-east'
        self.id = 0
        self.owner = MockMember()

    def get_member(self, *args):
        return None

class MockMessage(object):
    def __init__(self, sends, content=None):
        self.id = 0
        self.sends = sends
        self._state = 0
        self.guild = MockGuild()
        self.author = MockMember()
        self.channel = MockChannel(sends)
        self.content = content
    async def add_reaction(self, *args):
        pass

class Api(Cog):

    def __init__(self, bot):
        self.bot = bot

    @asyncio.coroutine
    def handle_request(self, pub, msg):
        try:
            resp = yield from getattr(self, msg['method'])(*msg['args'])
        except Exception as e:
            traceback.print_exc()
            print(f"caught {e} while handling {msg['topic']}s request")
            resp = '{"message": "' + str(e) + '"}'
        yield from pub.send((str(msg['topic']) + ' ' + str(resp)).encode())

    async def fetch_user_dict(self, id):
        usr = await self.bot.fetch_user(int(id))
        avatar = usr.avatar_url or usr.default_avatar_url
        return json.dumps({'name': usr.name, 'avatar_url': str(avatar)})

    async def interpret(self, message):
        args = message.split()
        for cmd in self.bot.commands:
            if args[0][1:] in cmd.aliases + [cmd.name]:
                command = cmd
                break
        else:
            return json.dumps({})
        sends = []
        ctx = Context(**{
            'message': MockMessage(sends, content=message),
            'bot': self.bot,
            'args': args[1:],
            'prefix': message[0],
            'command': command,
        })
        await ctx.invoke(command, *args[1:])
        return json.dumps({'response': '\n'.join(sends)})

def setup(bot):
    bot.add_cog(Api(bot))
