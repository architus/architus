import discord
from discord.ext import commands
from discord.ext.commands import Bot
import asyncio
import zmq
import zmq.asyncio
import json

from multiprocessing import Pipe

from src.user_command import UserCommand
from src.config import get_session
from src.models import Command


class CoolBot(Bot):

    def __init__(self, **kwargs):
        self.user_commands = {}
        self.session = get_session()
        super().__init__(**kwargs)

    @commands.command()
    async def test(ctx):
        print(ctx.message.content)

    async def fetch_user_dict(self, id):
        usr = await self.fetch_user(int(id))
        avatar = usr.avatar_url or usr.default_avatar_url
        return json.dumps({'name' : usr.name, 'avatar_url' : str(avatar)})


    @asyncio.coroutine
    def poll_requests(self, ctx):
        pub = ctx.socket(zmq.PUB)
        pub.bind("tcp://127.0.0.1:7208")
        while True:
            if hasattr(self, 'q') and not self.q.empty():
                #msg = yield from sub.recv_json()
                msg = json.loads(self.q.get())
                self.loop.create_task(self.handle_request(pub, msg))
            yield from asyncio.sleep(.01)

    @asyncio.coroutine
    def handle_request(self, pub, msg):
        try:
            resp = (yield from getattr(self, msg['method'])(msg['arg']))
        except Exception as e:
            print(f"caught {e} while handling {msg['topic']}s request")
            resp = '{"message": "' + str(e) + '"}'
        print("sending back " + str(resp))
        yield from pub.send((str(msg['topic']) + ' ' + str(resp)).encode())

    async def on_message(self, msg):
        print('Message from {0.author}: {0.content}'.format(msg))

        if msg.author == self.user:
            return

        # check for real commands
        await self.process_commands(msg)
        # check for user commands
        for command in self.user_commands[msg.guild.id]:
            if (command.triggered(msg.content)):
                await command.execute(msg, self.session)
                break

    async def on_ready(self):
        await self.initialize_user_commands()
        print('Logged on as {0}!'.format(self.user))
        await self.change_presence(activity=discord.Activity(name="the tragedy of darth plagueis the wise", type=3))

    async def initialize_user_commands(self):
        command_list = self.session.query(Command).all()
        for guild in self.guilds:
            self.user_commands.setdefault(int(guild.id), [])
        for command in command_list:
            self.user_commands.setdefault(command.server_id, [])
            self.user_commands[command.server_id].append(UserCommand(
                        command.trigger.replace(str(command.server_id), '', 1),
                        command.response, command.count,
                        self.get_guild(command.server_id),
                        command.author_id))
        for guild, cmds in self.user_commands.items():
            self.user_commands[guild].sort()

BOT_PREFIX = ("?", "!")
coolbot = CoolBot(command_prefix=BOT_PREFIX)

coolbot.load_extension('src.commands.schedule_command')
coolbot.load_extension('src.commands.eight_ball_command')
coolbot.load_extension('src.commands.settings_command')
coolbot.load_extension('src.commands.quote_command')
coolbot.load_extension('src.commands.set_command')
coolbot.load_extension('src.guild_settings')
ctx = zmq.asyncio.Context()
coolbot.loop.create_task(coolbot.poll_requests(ctx))

if __name__ == '__main__':
    from src.config import secret_token
    coolbot.run(secret_token)

