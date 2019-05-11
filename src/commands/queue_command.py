
from src.commands.abstract_command import abstract_command
import discord

class queue_command(abstract_command):

    def __init__(self):
        super().__init__("queue", aliases=['q'])

    async def exec_cmd(self, **kwargs):
        players = kwargs['players']
        settings = kwargs['settings']

        if not settings.music_enabled:
            return True

        await self.client.send_typing(self.channel)
        player = players[self.server.id]
        await self.client.send_message(self.channel, embed=player.qembed())

        return True

    def get_help(self, **kwargs):
        return "List songs in queue"

    def get_usage(self):
        return ""
