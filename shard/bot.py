import asyncio
import os

from discord.ext.commands import Bot
import discord

from src.utils import guild_to_message, guild_to_dict
from lib.config import get_session, secret_token, logger, AsyncConnWrapper
# TODO: Get rid of this stuff
from lib.ipc import async_rpc_server
from lib.ipc.async_emitter import Emitter
from lib.hoar_frost import HoarFrostGenerator
from lib.ipc import grpc_client, manager_pb2 as message


class Architus(Bot):

    def __init__(self, **kwargs):
        self.session = get_session()
        self.asyncpg_wrapper = AsyncConnWrapper()
        self.deletable_messages = []

        self.hoarfrost_gen = HoarFrostGenerator()

        logger.debug("registering with manager...")
        manager_client = grpc_client.get_blocking_client()
        shard_info = manager_client.register(message.RegisterRequest())
        self.shard_id = shard_info.shard_id
        shard_dict = {'shard_id': shard_info.shard_id, 'shard_count': shard_info.shard_count}
        logger.info(f"Got shard_id {self.shard_id}")

        kwargs.update(shard_dict)
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

        self.manager_client = grpc_client.get_async_client()
        print("Got manager: {}", self.manager_client)

        self.loop.create_task(self.emitter.connect())

        super().run(token)

    async def on_message(self, msg):
        """Execute commands, then trigger autoresponses"""
        logger.info('Message from {0.author} in {0.guild.name}: {0.content}'.format(msg))

        if msg.author == self.user:
            return

        # check for real commands
        await self.process_commands(msg)

    async def on_ready(self):
        """pull autoresponses from the db, then set activity"""
        await self.asyncpg_wrapper.connect()
        logger.info('Logged on as {0}!'.format(self.user))
        await self.change_presence(activity=discord.Activity(
            name=f"the tragedy of darth plagueis the wise {self.shard_id}", type=2))
        await self.manager_client.guild_update(iter(self.guilds_as_message))

    async def on_guild_join(self, guild):
        logger.info(f" -- JOINED NEW GUILD: {guild.name} -- ")
        await self.manager_client.guild_update(iter(self.guilds_as_message))

    @property
    def settings(self):
        return self.cogs['GuildSettings']

    @property
    def guilds_as_message(self):
        guilds = []
        for guild in self.guilds:
            guild_message = guild_to_message(guild)
            guild_message.shard_id = self.shard_id
            guild_message.admin_ids.extend(self.settings[guild].admins_ids)
            guilds.append(guild_message)
        return guilds

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
            await self.manager_client.checkin(message.ShardID(shard_id=self.shard_id))

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


architus = Architus(command_prefix=command_prefix, max_messages=10000)

for ext in (e for e in os.listdir("src/ext") if e.endswith(".py")):
    architus.load_extension(f"src.ext.{ext[:-3]}")

architus.load_extension('src.emoji_manager')
architus.load_extension('src.api.api')
architus.load_extension('src.guild_settings')

if __name__ == '__main__':
    architus.run(secret_token)
