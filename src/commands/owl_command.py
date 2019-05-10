from src.commands.abstract_command import abstract_command
import discord

class owl_command(abstract_command):

    def __init__(self):
        super().__init__("owl")

    async def exec_cmd(self, **kwargs):
        await self.client.send_message(self.channel, "hello")

    def get_help(self):
        return ""
    def get_usage(self):
        return ""
