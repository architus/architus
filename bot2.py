import discord
from discord.ext import commands
from discord.ext.commands import Bot
import asyncio

from src.config import secret_token

class AutBot(Bot):

    @commands.command()
    async def test(ctx):
        print(ctx.message.content)

    async def on_ready(self):
        self.add_command(self.test)
        print('Logged on as {0}!'.format(self.user))
        await self.change_presence(activity=discord.Activity(name="the tragedy of darth plagueis the wise", type=3))

    async def on_message(self, message):
        await self.process_commands(message)
        print('Message from {0.author}: {0.content}'.format(message))

@asyncio.coroutine
def api_listener():
    context = zmq.asyncio.Context()
    socket = context.socket(zmq.REP)
    socket.bind('tcp://127.0.0.1:7100')
    while True:
        msg = yield from socket.recv_string()

        usr = yield from client.fetch_user(int(msg))
        yield from socket.send_string(usr.name)

BOT_PREFIX = ("?", "!")
autbot = AutBot(command_prefix=BOT_PREFIX)

autbot.load_extension('src.commands.schedule_command')

autbot.run(secret_token)
