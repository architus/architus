from discord import Guild

from src.utils import channel_to_dict, member_to_dict, role_to_dict


class Pools:
    def __init__(self, bot):
        self.bot = bot

    def get_all_emoji(self, guild: Guild):
        try:
            emoji_manager = self.bot.get_cog("Emoji Manager").managers[guild.id]
        except KeyError:
            return []

        return [e.as_dict() for e in emoji_manager.emojis]

    def get_all_channels(self, guild: Guild):
        return [channel_to_dict(ch) for ch in guild.channels]

    def get_all_roles(self, guild: Guild):
        return [role_to_dict(r) for r in guild.roles]

    def get_all_members(self, guild: Guild):
        return [member_to_dict(m) for m in guild.members]

    def get_all_responses(self, guild: Guild):
        try:
            auto_responses = self.bot.get_cog("Auto Responses").responses[guild.id]
        except KeyError:
            return []
        return [r.as_dict() for r in auto_responses.auto_responses]
