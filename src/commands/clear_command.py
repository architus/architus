from src.commands.abstract_command import abstract_command
import discord

class clear_command(abstract_command):

    def __init__(self):
        super().__init__("clear")

    async def exec_cmd(self, **kwargs):
        players = kwargs['players']
        settings = kwargs['settings']

        if not settings.music_enabled:
            return True

        player = players[self.server.id]

        await self.client.send_message(self.channel, "Removed %d songs from queue." % len(player.q))
        player.clearq()
        return True

    def get_help(self, **kwargs):
        return "Clear all songs from queue"

    def get_usage(self):
        return ""
