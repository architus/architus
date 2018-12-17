
from src.server_settings import server_settings
from src.commands.abstract_command import abstract_command
import time
import discord

class settings_command(abstract_command):

    def __init__(self):
        super().__init__("settings")

    async def exec_cmd(self, **kwargs):
        session = kwargs['session']
        settings = server_settings(session, self.server)
        if self.author.id not in settings.admins_ids:
            self.client.send_message(self.channel, 'nope, sorry')
            return
        if 'defaultrole' in self.content.lower():
            settings.default_role_id = self.message.role_mentions[0].id
        elif 'bot-commands' in self.content.lower():
            bc_channels = settings.bot_commands_channels
            new_channels = self.message.channel_mentions or [self.channel.id]

            for channel in new_channels:
                if channel.id in bc_channels:
                    bc_channels.remove(channel.id)
                else:
                    bc_channels.append(channel.id)
                print (bc_channels)
                settings.bot_commands_channels = bc_channels

        elif 'aut-emoji' in self.content.lower():
            settings.aut_emoji = self.args[2]
        elif 'nice-emoji' in self.content.lower():
            settings.nice_emoji = self.args[2]
        elif 'toxic-emoji' in self.content.lower():
            settings.toxic_emoji = self.args[2]
        elif 'bot-emoji' in self.content.lower():
            settings.bot_emoji = self.args[2]
        elif 'norm-emoji' in self.content.lower():
            settings.norm_emoji = self.args[2]
        elif 'edit-emoji' in self.content.lower():
            settings.edit_emoji = self.args[2]
        elif 'repost-deletes' in self.content.lower():
            settings.repost_del_msg = True if self.args[2] in ['1','True','true','yes'] else False


        elif 'admin' in self.content.lower():
            if self.message.mentions[0].id in settings.admins_ids:
                settings.admins_ids.remove(self.message.mentions[0].id)
            else:  settings.admins_ids += [self.message.mentions[0].id]

        await self.client.send_message(self.channel, str(settings._settings_dict))
    def get_help(self):
        return ""

    def get_usage(self):
        return ''
