from discord.ext import commands
from discord import ChannelType


class LogCog(commands.Cog):

    def __init__(self, bot):
        self.bot = bot
        self.hoarfrost = bot.hoarfrost_gen

    @commands.Cog.listener()
    async def on_guild_update(self, before, after):
        await self.bot.emitter.emit(
            'gateway.on_guild_update',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': before.id,
                'discord_object': before.id,
                'private': False,
                'reversible': False,  # should be possible eventually
                'action_number': 1,
                'agent_id': None,
                'subject_id': before.id,
                'data': {
                    'before': {
                        'name': before.name,
                        # ...
                    },
                    'after': {
                        'name': after.name,
                        # ...
                    },
                },
            }
        )

    @commands.Cog.listener()
    async def on_guild_available(self, guild):
        await self.bot.emitter.emit(
            'gateway.on_guild_available',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': guild.id,
                'discord_object': guild.id,
                'private': False,
                'reversible': False,
                'action_number': -1,
                'agent_id': None,
                'subject_id': guild.id,
                'data': {
                    'availible': True,
                },
            }
        )

    @commands.Cog.listener()
    async def on_guild_unavailable(self, guild):
        await self.bot.emitter.emit(
            'gateway.on_guild_unavailable',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': guild.id,
                'discord_object': guild.id,
                'private': False,
                'reversible': False,
                'action_number': -1,
                'agent_id': None,
                'subject_id': guild.id,
                'data': {
                    'availible': False,
                },
            }
        )

    @commands.Cog.listener()
    async def on_guild_role_create(self, role):
        await self.bot.emitter.emit(
            'gateway.on_guild_role_create',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': role.guild.id,
                'discord_object': role.id,
                'private': False,
                'reversible': False,  # should be possible eventually
                'action_number': 30,
                'agent_id': None,
                'subject_id': role.id,
                'data': {
                    'name': role.name,
                    # ...
                },
            }
        )

    @commands.Cog.listener()
    async def on_guild_role_delete(self, role):
        await self.bot.emitter.emit(
            'gateway.on_guild_role_delete',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': role.guild.id,
                'discord_object': role.id,
                'private': False,
                'reversible': False,  # should be possible eventually
                'action_number': 32,
                'agent_id': None,
                'subject_id': role.id,
                'data': {
                    'name': role.name,
                    # ...
                },
            }
        )

    @commands.Cog.listener()
    async def on_guild_role_update(self, before, after):
        await self.bot.emitter.emit(
            'gateway.on_guild_role_update',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': before.guild.id,
                'discord_object': before.id,
                'private': False,
                'reversible': False,  # should be possible eventually
                'action_number': 31,
                'agent_id': None,
                'subject_id': before.id,
                'data': {
                    'before': {
                        'name': before.name,
                        # ...
                    },
                    'after': {
                        'name': after.name,
                        # ...
                    },
                },
            }
        )

    @commands.Cog.listener()
    async def on_guild_emojis_update(self, guild, before, after):
        pass

    @commands.Cog.listener()
    async def on_guild_channel_create(self, channel):
        await self.bot.emitter.emit(
            'gateway.on_guild_channel_create',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': channel.guild.id,
                'discord_object': channel.id,
                'private': False,
                'reversible': False,  # should be possible eventually
                'action_number': 10,
                'agent_id': None,
                'subject_id': channel.id,
                'data': {
                    'name': channel.name,
                    # ...
                },
            }
        )

    @commands.Cog.listener()
    async def on_guild_channel_delete(self, channel):
        await self.bot.emitter.emit(
            'gateway.on_guild_channel_delete',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': channel.guild.id,
                'discord_object': channel.id,
                'private': False,
                'reversible': False,  # should be possible eventually
                'action_number': 12,
                'agent_id': None,
                'subject_id': channel.id,
                'data': {
                    'name': channel.name,
                    # ...
                },
            }
        )

    @commands.Cog.listener()
    async def on_guild_channel_update(self, before, after):
        await self.bot.emitter.emit(
            'gateway.on_guild_channel_update',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': before.guild.id,
                'discord_object': before.id,
                'private': False,
                'reversible': False,  # should be possible eventually
                'action_number': 11,
                'agent_id': None,
                'subject_id': before.id,
                'data': {
                    'before': {
                        'name': before.name,
                        # ...
                    },
                    'after': {
                        'name': after.name,
                        # ...
                    },
                },
            }
        )

    @commands.Cog.listener()
    async def on_bulk_message_delete(self, messages):
        pass

    @commands.Cog.listener()
    async def on_message_edit(self, before, after):
        await self.bot.emitter.emit(
            'gateway.on_message_edit',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': before.guild.id,
                'discord_object': before.id,
                'private': False,
                'reversible': False,
                'action_number': 3002,
                'agent_id': None,
                'subject_id': before.id,
                'data': {
                    'before': {
                        'content': before.content,
                        # ...
                    },
                    'after': {
                        'content': after.content,
                        # ...
                    },
                },
            }
        )

    @commands.Cog.listener()
    async def on_message(self, msg):
        await self.bot.emitter.emit(
            'gateway.on_message',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': msg.channel.guild.id,
                'discord_object': msg.id,
                'private': msg.channel.type == ChannelType.private,
                'reversible': True,
                'action_number': 3001,
                'agent_id': msg.author.id,
                'subject_id': msg.channel.id,
                'data': {
                    'content': msg.content,
                    'created_at': msg.created_at.isoformat(),
                },
            }
        )

    @commands.Cog.listener()
    async def on_message_delete(self, msg):
        await self.bot.emitter.emit(
            'gateway.on_message_delete',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': msg.channel.guild.id,
                'discord_object': msg.id,
                'private': msg.channel.type == ChannelType.private,
                'reversible': True,
                'action_number': 3003,
                'agent_id': msg.author.id,
                'subject_id': msg.id,
                'data': None,
            }
        )

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        await self.bot.emitter.emit(
            'gateway.on_reation_add',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': react.message.channel.guild.id,
                'discord_object': None,
                'private': react.message.channel.type == ChannelType.private,
                'reversible': True,
                'action_number': 3100,
                'agent_id': user.id,
                'subject_id': react.message.id,
                'data': {
                    'emoji': str(react.emoji),
                },
            }
        )

    @commands.Cog.listener()
    async def on_reaction_remove(self, react, user):
        await self.bot.emitter.emit(
            'gateway.on_reation_remove',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': react.message.channel.guild.id,
                'discord_object': None,
                'private': react.message.channel.type == ChannelType.private,
                'reversible': False,
                'action_number': 3101,
                'agent_id': user.id,
                'subject_id': react.message.id,
                'data': {
                    'emoji': str(react.emoji),
                },
            }
        )

    @commands.Cog.listener()
    async def on_reaction_clear(self, msg, react):
        pass

    @commands.Cog.listener()
    async def on_webhooks_update(self, channel):
        pass

    @commands.Cog.listener()
    async def on_member_join(self, member):
        await self.bot.emitter.emit(
            'gateway.on_member_join',
            {
                'id': self.hoarfrost.generate(),
                'guild_id': member.guild.id,
                'discord_object': member.id,
                'private': False,
                'reversible': False,
                'action_number': -1,
                'agent_id': None,
                'subject_id': member.id,
                'data': {
                    'name': member.name,
                    # ...
                },
            }
        )

    @commands.Cog.listener()
    async def on_member_remove(self, member):
        pass

    @commands.Cog.listener()
    async def on_member_update(self, before, after):
        pass

    @commands.Cog.listener()
    async def on_member_ban(self, guild, user):
        pass

    @commands.Cog.listener()
    async def on_member_unban(self, guild, user):
        pass

    @commands.Cog.listener()
    async def on_voice_state_update(self, member, before, after):
        pass


def setup(bot):
    pass
    # bot.add_cog(LogCog(bot))
