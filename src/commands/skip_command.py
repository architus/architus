from src.commands.abstract_command import abstract_command
import discord

class skip_command(abstract_command):

    def __init__(self):
        super().__init__("skip")

    async def exec_cmd(self, **kwargs):
        players = kwargs['players']
        settings = kwargs['settings']

        if not settings.music_enabled:
            return True

        player = players[self.server.id]

        name = await player.skip()
        if (name):
            await self.client.send_message(self.channel, "🎶 **Now playing:** *%s*" % name)
        else:
            await self.client.send_message(self.channel, "No songs left. nice job. bye.")

        return True

    def get_help(self, **kwargs):
        return "Skip currently playing song"

    def get_usage(self):
        return ""
