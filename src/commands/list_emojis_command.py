from src.commands.abstract_command import abstract_command
import discord

class list_emojis_command(abstract_command):

    def __init__(self):
        super().__init__("list_emojis", aliases=['emojis', 'emotes', 'emoji', 'emote'])

    async def exec_cmd(self, **kwargs):
        emoji_managers = kwargs['emoji_managers']
        settings = kwargs['settings']
        
        message = '```\n • ' + '\n • '.join(emoji_managers[self.server.id].list_unloaded()) + '```\n'
        message += "Enclose the name (case sensitive) of cached emoji in `:`s to auto-load it into a message"

        await self.channel.send(message)

        return True

    def get_brief(self, **kwargs):
        return "List currently cached emojis"

    def get_help(self, **kwargs):
        return "List currently cached emojis. Enclose the name (case sensitive) of cached emoji in `:`s to auto-load it into a message"

    def get_usage(self):
        return ""
