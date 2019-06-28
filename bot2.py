import discord
from discord.ext import commands
from discord.ext.commands import Bot
import asyncio
import zmq
import zmq.asyncio
import json
import websockets
import ssl
from pytz import timezone

from multiprocessing import Pipe

from src.user_command import UserCommand
from src.config import get_session
from src.models import Command

starboarded_messages = []


class CoolBot(Bot):

    def __init__(self, **kwargs):
        self.user_commands = {}
        self.session = get_session()
        super().__init__(**kwargs)

    def run(self, token, q=None):
        self.q = q

        ctx = zmq.asyncio.Context()
        self.loop.create_task(self.poll_requests(ctx))
        try:
            ssl_context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
            ssl_context.load_cert_chain('certificate.pem', 'privkey.pem')

            start_server = websockets.serve(self.get_cog("Api").handle_socket, '0.0.0.0', 8300, ssl=ssl_context)
        except FileNotFoundError:
            print("SSL certs not found, websockets running in insecure mode")
            start_server = websockets.serve(self.get_cog("Api").handle_socket, '0.0.0.0', 8300)
        asyncio.async(start_server)
        super().run(token)

    @asyncio.coroutine
    def poll_requests(self, ctx):
        api = self.get_cog('Api')
        pub = ctx.socket(zmq.PUB)
        pub.bind("tcp://127.0.0.1:7200")
        while True:
            if not self.q.empty():
                msg = json.loads(self.q.get())
                self.loop.create_task(api.handle_request(pub, msg))
            yield from asyncio.sleep(.01)

    async def on_reaction_add(self, react, user):
        if user == self.user:
            return
        settings = self.guild_settings.get_guild(react.message.guild, session=self.session)
        if settings.starboard_emoji in str(react.emoji):
            if react.count == settings.starboard_threshold:
                await self.starboard_post(react.message, react.message.guild)

    async def on_message(self, msg):
        print('Message from {0.author} in {0.guild.name}: {0.content}'.format(msg))

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

    @property
    def guild_settings(self):
        return self.get_cog('GuildSettings')

    async def starboard_post(self, message, guild):
        starboard_ch = discord.utils.get(guild.text_channels, name='starboard')
        if message.id in starboarded_messages or not starboard_ch or message.author == self.user:
            return
        print("Starboarding message: " + message.content)
        starboarded_messages.append(message.id)
        utc = message.created_at.replace(tzinfo=timezone('UTC'))
        est = utc.astimezone(timezone('US/Eastern'))
        em = discord.Embed(title=est.strftime("%Y-%m-%d %I:%M %p"), description=message.content, colour=0x42f468)
        em.set_author(name=message.author.display_name, icon_url=message.author.avatar_url)
        if message.embeds:
            em.set_image(url=message.embeds[0].url)
        elif message.attachments:
            em.set_image(url=message.attachments[0].url)
        await starboard_ch.send(embed=em)

BOT_PREFIX = ("?", "!")
coolbot = CoolBot(command_prefix=BOT_PREFIX)

coolbot.load_extension('src.commands.schedule_command')
coolbot.load_extension('src.commands.eight_ball_command')
coolbot.load_extension('src.commands.settings_command')
coolbot.load_extension('src.commands.quote_command')
coolbot.load_extension('src.commands.set_command')
coolbot.load_extension('src.commands.play_command')
coolbot.load_extension('src.commands.messagecount_command')
coolbot.load_extension('src.commands.role_command')
coolbot.load_extension('src.commands.gulag_command')
coolbot.load_extension('src.emoji_manager')
coolbot.load_extension('src.api.api')
coolbot.load_extension('src.guild_settings')

if __name__ == '__main__':
    from src.config import secret_token
    coolbot.run(secret_token)

