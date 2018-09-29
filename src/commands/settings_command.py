
from src.server_settings import server_settings
from src.commands.abstract_command import abstract_command
import time
import discord

class settings_command(abstract_command):

    def __init__(self):
        super().__init__("settings")

    async def exec_cmd(self, **kwargs):
        session = kwargs['session']
        settings = server_settings(session, self.server.id)
        if 'defaultrole' in self.message.clean_content.lower():
            settings.default_role_id = self.message.role_mentions[0].id
        elif 'botcommands' in self.message.clean_content.lower():
            bc_channels = settings.bot_commands_channels
            new_channels = self.message.channel_mentions or [self.channel.id]

            for channel in new_channels:
                if channel.id in bc_channels:
                    bc_channels.remove(channel.id)
                else:
                    bc_channels.append(channel.id)
                print (bc_channels)
                settings.bot_commands_channels = bc_channels

        await self.client.send_message(self.channel, str(settings._settings_dict))
    def get_help(self):
        return ""

    def get_usage(self):
        return ''
