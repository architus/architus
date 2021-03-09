from discord import Guild
from discord.errors import NotFound

from contextlib import suppress

from src.utils import channel_to_dict, member_to_dict, role_to_dict, user_to_dict, guild_to_dict


class Pools:
    def __init__(self, bot):
        self.nonexistant = []
        self.bot = bot

    async def get_all_emoji(self, guild: Guild):
        try:
            emoji_manager = self.bot.get_cog("Emoji Manager").managers[guild.id]
        except KeyError:
            return []

        return [await e.as_dict_url() for e in emoji_manager.emojis]

    async def get_emoji(self, guild: Guild, emoji_id, fetch=False):
        if fetch:
            return {}
        try:
            emoji_manager = self.bot.get_cog("Emoji Manager").managers[guild.id]
        except KeyError:
            raise
        emoji = emoji_manager.find_emoji(a_id=int(emoji_id))

        return await emoji.as_dict_url()

    def get_all_channels(self, guild: Guild):
        return [channel_to_dict(ch) for ch in guild.channels]

    def get_all_roles(self, guild: Guild):
        return [role_to_dict(r) for r in guild.roles]

    def get_all_members(self, guild: Guild):
        return [member_to_dict(m) for m in guild.members]

    async def get_member(self, guild: Guild, member_id, fetch=False):
        if member_id in self.nonexistant:
            raise Exception(f"unknown member {member_id}")
        member = guild.get_member(int(member_id))
        if member is None and fetch:
            try:
                member = await guild.fetch_member(int(member_id))
            except NotFound:
                self.nonexistant.append(member_id)
        if member is None:
            raise Exception(f"unknown member {member_id}")
        return member_to_dict(member)

    async def get_user(self, user_id, fetch=False):
        if user_id in self.nonexistant:
            raise Exception(f"unknown user {user_id}")
        user = self.bot.get_user(int(user_id))
        if user is None and fetch:
            try:
                user = await self.bot.fetch_user(int(user_id))
            except NotFound:
                self.nonexistant.append(user_id)
        if user is None:
            raise Exception(f"unknown user {user_id}")
        return user_to_dict(user)

    def get_all_responses(self, guild: Guild):
        try:
            auto_responses = self.bot.get_cog("Auto Responses").responses[guild.id]
        except KeyError:
            return []
        return [r.as_dict() for r in auto_responses.auto_responses]

    async def get_guild(self, member_id: int, guild_id: int, fetch=False):
        guild = self.bot.get_guild(int(guild_id))
        if guild is None and fetch:
            with suppress(NotFound):
                guild = await self.bot.fetch_guild(int(guild_id))

        if guild is None:
            raise Exception(f"unknown guild {guild_id}, attempted fetch: {fetch}")

        member = guild.get_member(int(member_id))
        settings = self.bot.settings[guild]
        g = guild_to_dict(guild)
        g.update({
            'owner': int(g['owner_id']) == int(member_id),
            'has_architus': True,
            'architus_admin': int(member_id) in settings.admins_ids,
            'permissions': member.guild_permissions.value,
        })
        return g
