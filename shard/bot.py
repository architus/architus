import discord
from discord.ext.commands import Bot
from datetime import datetime
import asyncio
import os
from pytz import timezone

from src.user_command import UserCommand
from src.smart_message import smart_message
from lib.config import get_session, secret_token
from lib.models import Command
from lib.ipc import async_rpc_server, async_rpc_client, blocking_rpc_client

starboarded_messages = []


class Architus(Bot):

    def __init__(self, **kwargs):
        self.user_commands = {}
        self.session = get_session()
        self.tracked_messages = {}
        self.deletable_messages = []

        manager_client = blocking_rpc_client.shardRPC()
        # wait for manager to come up; this is scuffed
        import time
        time.sleep(2)
        print("asking for shard id")

        shard_info, sc = manager_client.register(routing_key='manager_rpc')
        self.shard_id = shard_info['shard_id']
        print(f"Got shard_id {self.shard_id}")

        kwargs.update(shard_info)
        super().__init__(**kwargs)

    def run(self, token):
        self.loop.create_task(self.list_guilds())
        self.loop.create_task(
            async_rpc_server.start_server(
                self.loop,
                f'shard_rpc_{self.shard_id}',
                self.get_cog('Api').api_entry
            )
        )

        self.manager_client = async_rpc_client.shardRPC(self.loop, default_key='manager_rpc')
        self.loop.create_task(self.manager_client.connect())

        super().run(token)

    async def on_reaction_add(self, react, user):
        """Check if message should be added to starboard or an edit dialog should be posted"""
        if user == self.user:
            return
        guild = react.message.guild
        settings = self.settings[guild]
        if settings.starboard_emoji in str(react.emoji):
            if react.count == settings.starboard_threshold:
                await self.starboard_post(react.message, guild)

        if settings.edit_emoji in str(react.emoji):
            sm = self.tracked_messages.get(react.message.id)
            if sm:
                await sm.add_popup(react.message.channel)

    async def on_reaction_remove(self, react, user):
        settings = self.settings[react.message.guild]
        if settings.edit_emoji in str(react.emoji) and react.count == 0:
            sm = self.tracked_messages[react.message.id]
            await sm.delete_popup()

    async def on_message_delete(self, msg):
        settings = self.settings[msg.guild]
        if msg.id in self.deletable_messages:
            self.deletable_messages.remove(msg.id)
            return
        if msg.author != self.user and settings.repost_del_msg:
            utc = msg.created_at.replace(tzinfo=timezone('UTC'))
            est = utc.astimezone(timezone('US/Eastern'))
            em = discord.Embed(title=est.strftime("%Y-%m-%d %I:%M %p"), description=msg.content, colour=0x42f468)
            em.set_author(name=msg.author.display_name, icon_url=msg.author.avatar_url)
            repost = await msg.channel.send(embed=em)
            if msg.id in self.tracked_messages:
                self.tracked_messages[repost.id] = self.tracked_messages[msg.id]

    async def on_message_edit(self, before, after):
        """Adds message to the store of edits"""
        if before.author == self.user:
            return

        sm = self.tracked_messages.get(before.id)
        # have to manually give datetime.now() cause discord.py is broken
        if sm and sm.add_edit(before, after, datetime.now()):
            await sm.edit_popup()
            return
        sm = smart_message(before)
        sm.add_edit(before, after, datetime.now())
        self.tracked_messages[before.id] = sm

    async def on_message(self, msg):
        """Execute commands, then trigger autoresponses"""
        print('Message from {0.author} in {0.guild.name}: {0.content}'.format(msg))

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
        print('Logged on as {0}!'.format(self.user))
        await self.change_presence(activity=discord.Activity(name=f"shard id: {self.shard_id}", type=3))

    async def on_guild_join(self, guild):
        print(f" -- JOINED NEW GUILD: {guild.name} -- ")
        self.user_commands.setdefault(guild.id, [])

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
        return self.get_cog('GuildSettings')

    async def list_guilds(self):
        """Update the manager with the guilds that we know about"""
        await self.wait_until_ready()
        while not self.is_closed():
            print("Current guilds:")
            guilds = []
            for guild in self.guilds:
                me = guild.get_member(self.user.id)
                if me.display_name == 'archit.us':
                    await me.edit(nick='architus')
                print("{} - {} ({})".format(guild.name, guild.id, guild.member_count))
                settings = self.settings[guild]
                guilds.append({
                    'id': guild.id,
                    'name': guild.name,
                    'icon': guild.icon,
                    'member_count': guild.member_count,
                    'admin_ids': settings.admins_ids
                })
                # TODO this should happen on update not every 600 seconds
            await self.manager_client.call('guild_update', self.shard_id, guilds)
            await asyncio.sleep(600)

    async def starboard_post(self, message, guild):
        """Post a message in the starboard channel"""
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


architus = Architus(command_prefix=('!', '?'))

for ext in (e for e in os.listdir("src/ext") if e.endswith(".py")):
    architus.load_extension(f"src.ext.{ext[:-3]}")

architus.load_extension('src.emoji_manager')
architus.load_extension('src.api.api')
architus.load_extension('src.guild_settings')

if __name__ == '__main__':
    architus.run(secret_token)
