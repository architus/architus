import json
import traceback
import asyncio
import secrets
import websockets
from discord.ext.commands import Cog, Context


class MockMember(object):
    def __init__(self):
        self.id = 0
class MockRole(object):
    pass

class MockChannel(object):
    def __init__(self, sends, reactions):
        self.sends = sends
        self.reactions = reactions
    async def send(self, *args):
        for thing in args:
            self.sends.append(thing)
        return MockMessage(0, self.sends, self.reactions, 0)

class MockGuild(object):
    def __init__(self, id):
        self.region = 'us-east'
        self.id = int(id)
        self.owner = MockMember()
        self.default_role = MockRole()
        self.default_role.mention = "@everyone"
        self.emojis = []

    def get_member(self, *args):
        return None

class MockMessage(object):
    def __init__(self, id, sends, reactions, guild_id, content=None):
        self.id = id
        self.sends = sends
        self.reactions = reactions
        self._state = MockChannel(sends, reactions)
        self.guild = MockGuild(guild_id)
        self.author = MockMember()
        self.channel = MockChannel(sends, reactions)
        self.content = content
    async def add_reaction(self, emoji):
        self.reactions.append((self.id, emoji))

class Api(Cog):

    def __init__(self, bot):
        self.bot = bot
        self.fake_messages = {}

    async def handle_socket(self, websocket, path):
        while True:
            try:
                data = json.loads(await websocket.recv())
                resp = await self.interpret(
                        data['guild_id'],
                        data['message'],
                        data['message_id']
                )
            except Exception as e:
                traceback.print_exc()
                print(f"caught {e} while handling websocket request")
                resp = {'message': str(e)}
            await websocket.send(json.dumps(resp))

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
        return json.dumps({'name': usr.name, 'avatar': usr.avatar})

    async def reload_extension(self, extension_name):
        name = extension_name.replace('-', '.')
        print(f"reloading extention: {name}")
        self.bot.reload_extension(name)
        return json.dumps({})

    async def interpret(self, guild_id, message, message_id):
        self.fake_messages[guild_id] = {
            'content': message,
            'reactions': [],
            'message_id': message_id,
            'guild_id': guild_id,
        }

        # search for builtin commands
        command = None
        args = message.split()
        for cmd in self.bot.commands:
            if args[0][1:] in cmd.aliases + [cmd.name]:
                command = cmd
                break
        sends = []
        reactions = []
        mock_message = MockMessage(message_id, sends, reactions, guild_id, content=message)

        self.bot.user_commands.setdefault(int(guild_id), [])
        if command:
            # found builtin command, creating fake context
            ctx = Context(**{
                'message': mock_message,
                'bot': self.bot,
                'args': args[1:],
                'prefix': message[0],
                'command': command,
            })
            await ctx.invoke(command, *args[1:])
        else:
            # check for user set commands in this "guild"
            for command in self.bot.user_commands[mock_message.guild.id]:
                if (command.triggered(mock_message.content)):
                    await command.execute(mock_message, self.bot.session)
                    break
        new_id = secrets.randbits(24) | 1
        resp = {
            'content': '\n'.join(sends),
            'reactions': [(new_id if r[0] == 0 else message_id, r[1]) for r in reactions],
            'message_id': new_id,
            'guild_id': guild_id,
            '_module': 'interpret'
        }
        self.fake_messages[resp['message_id']] = resp
        self.fake_messages[resp['message_id']]['from_autbot'] = True
        return resp


def setup(bot):
    bot.add_cog(Api(bot))
