import asyncio
import os

from discord.ext.commands import Bot
import discord

from src.user_command import UserCommand
from src.utils import guild_to_dict
from lib.config import get_session, secret_token, logger
from lib.models import Command
from lib.ipc import async_rpc_server, async_rpc_client, blocking_rpc_client
from lib.ipc.async_emitter import Emitter
from lib.hoar_frost import HoarFrostGenerator


class Architus(Bot):

    def __init__(self, **kwargs):
        self.user_commands = {}
        self.session = get_session()
        self.deletable_messages = []

        self.hoarfrost_gen = HoarFrostGenerator()

        manager_client = blocking_rpc_client.shardRPC()
        # wait for manager to come up; this is scuffed
        import time
        time.sleep(2)
        logger.debug("asking for shard id")

        shard_info, sc = manager_client.register(routing_key='manager_rpc')
        self.shard_id = shard_info['shard_id']
        logger.info(f"Got shard_id {self.shard_id}")

        kwargs.update(shard_info)
        super().__init__(**kwargs)

    def run(self, token):
        self.emitter = Emitter(self.loop)
        self.loop.create_task(self.list_guilds())
        self.loop.create_task(self.heartbeat())
        self.loop.create_task(
            async_rpc_server.start_server(
                self.loop,
                f'shard_rpc_{self.shard_id}',
                self.cogs['Api'].api_entry
            )
        )

        self.manager_client = async_rpc_client.shardRPC(self.loop, default_key='manager_rpc')
        self.loop.create_task(self.manager_client.connect())

        self.loop.create_task(self.emitter.connect())

        super().run(token)

    async def on_message(self, msg):
        """Execute commands, then trigger autoresponses"""
        logger.info('Message from {0.author} in {0.guild.name}: {0.content}'.format(msg))

        if msg.author == self.user:
            return

        # check for real commands
        await self.process_commands(msg)
        # check for user commands
        for command in self.user_commands[msg.guild.id]:
            if (command.triggered(msg.content)):
                await command.execute(msg)
                break

    async def on_ready(self):
        """pull autoresponses from the db, then set activity"""
        await self.initialize_user_commands()
        logger.info('Logged on as {0}!'.format(self.user))
        await self.change_presence(activity=discord.Activity(
            name=f"the trageedy of darth plagueis the wise {self.shard_id}", type=2))
        await self.manager_client.guild_update(self.shard_id, self.guilds_as_dicts)

    async def on_guild_join(self, guild):
        logger.info(f" -- JOINED NEW GUILD: {guild.name} -- ")
        self.user_commands.setdefault(guild.id, [])
        await self.manager_client.guild_update(self.shard_id, self.guilds_as_dicts)

    async def initialize_user_commands(self):
        command_list = self.session.query(Command).all()
        for guild in self.guilds:
            self.user_commands.setdefault(int(guild.id), [])
        for command in command_list:
            self.user_commands.setdefault(command.server_id, [])
            self.user_commands[command.server_id].append(UserCommand(
                self.session,
                self,
                command.trigger.replace(str(command.server_id), '', 1),
                command.response, command.count,
                self.get_guild(command.server_id),
                command.author_id))
            for guild, cmds in self.user_commands.items():
                self.user_commands[guild].sort()

    @property
    def settings(self):
        return self.cogs['GuildSettings']

    @property
    def guilds_as_dicts(self):
        guilds = []
        for guild in self.guilds:
            guild_dict = guild_to_dict(guild)
            guild_dict.update({'admin_ids': self.settings[guild].admins_ids})
            guilds.append(guild_dict)
        return guilds

    async def heartbeat(self):
        await self.wait_until_ready()
        while not self.is_closed():
            await asyncio.sleep(0.5)
            await self.manager_client.checkin(self.shard_id)

    async def list_guilds(self):
        """Update the manager with the guilds that we know about"""
        await self.wait_until_ready()
        while not self.is_closed():
            logger.info("Current guilds:")
            for guild in self.guilds:
                if guild.me.display_name == 'archit.us':
                    try:
                        await guild.me.edit(nick='architus')
                    except discord.Forbidden:
                        logger.warning(f"couldn't change nickname in {guild.name}")
                logger.info("{} - {} ({})".format(guild.name, guild.id, guild.member_count))
            await asyncio.sleep(600)


def command_prefix(bot: Architus, msg: discord.Message):
    return bot.settings[msg.guild].command_prefix


architus = Architus(command_prefix=command_prefix)

for ext in (e for e in os.listdir("src/ext") if e.endswith(".py")):
    architus.load_extension(f"src.ext.{ext[:-3]}")

architus.load_extension('src.emoji_manager')
architus.load_extension('src.api.api')
architus.load_extension('src.guild_settings')

if __name__ == '__main__':
    architus.run(secret_token)
