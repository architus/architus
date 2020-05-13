from discord import Guild

from src.utils import channel_to_dict


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

    def get_all_guilds(self):
        return []
