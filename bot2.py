import discord
from discord.ext import commands
from discord.ext.commands import Bot
import asyncio


class CoolBot(Bot):

    @commands.command()
    async def test(ctx):
        print(ctx.message.content)

    async def poll_pipe(self):
        print("running thing")
        while True:
            if self and hasattr(self, 'conn'):
                channel = self.get_channel(436189230390050830)
                if channel:
                    await channel.send(str(self.conn.recv()) + " in the bot")
                await asyncio.sleep(1)

    async def on_ready(self):
        self.add_command(self.test)
        print('Logged on as {0}!'.format(self.user))
        await self.change_presence(activity=discord.Activity(name="the tragedy of darth plagueis the wise", type=3))

    async def on_message(self, message):
        await self.process_commands(message)
        print('Message from {0.author}: {0.content}'.format(message))

BOT_PREFIX = ("?", "!")
coolbot = CoolBot(command_prefix=BOT_PREFIX)

coolbot.load_extension('src.commands.schedule_command')
coolbot.loop.create_task(coolbot.poll_pipe())

