from discord import Guild
from discord.errors import NotFound

from contextlib import suppress

from src.utils import channel_to_dict, member_to_dict, role_to_dict, user_to_dict


class Pools:
    def __init__(self, bot):
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
            return []
        emoji = emoji_manager.find_emoji(a_id=emoji_id)

        return (await emoji.as_dict_url()) if emoji else {}

    def get_all_channels(self, guild: Guild):
        return [channel_to_dict(ch) for ch in guild.channels]

    def get_all_roles(self, guild: Guild):
        return [role_to_dict(r) for r in guild.roles]

    def get_all_members(self, guild: Guild):
        return [member_to_dict(m) for m in guild.members]

    async def get_member(self, guild: Guild, member_id, fetch=False):
        member = guild.get_member(int(member_id))
        if member is None and fetch:
            with suppress(NotFound):
                member = await guild.fetch_member(int(member_id))
        return member_to_dict(member)

    async def get_user(self, user_id, fetch=False):
        user = self.bot.get_user(int(user_id))
        if user is None and fetch:
            with suppress(NotFound):
                user = await self.bot.fetch_user(int(user_id))
        return user_to_dict(user)

    def get_all_responses(self, guild: Guild):
        try:
            auto_responses = self.bot.get_cog("Auto Responses").responses[guild.id]
        except KeyError:
            return []
        return [r.as_dict() for r in auto_responses.auto_responses]
