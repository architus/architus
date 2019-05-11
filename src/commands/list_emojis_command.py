from src.commands.abstract_command import abstract_command
import discord

class list_emojis_command(abstract_command):

    def __init__(self):
        super().__init__("list_emojis", aliases=['emojis'])

    async def exec_cmd(self, **kwargs):
        emoji_managers = kwargs['emoji_managers']
        settings = kwargs['settings']

        await self.client.send_message(self.channel, '```' + '\n'.join(emoji_managers[self.server.id].list_unloaded()) + '```')

        return True

    def get_help(self, **kwargs):
        return "List currently caches emojis"

    def get_usage(self):
        return ""
