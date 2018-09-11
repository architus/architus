from src.commands.abstract_command import abstract_command
from src.models import User, Admin, Command
from discord import ChannelType
from src.smart_command import smart_command, VaguePatternError
import re
import discord

class set_command(abstract_command):

    def __init__(self):
        super().__init__("set")

    async def exec_cmd(self, **kwargs):
        self.session = kwargs['session']
        smart_commands = kwargs['smart_commands']
        admins = kwargs['admins']
        server = self.server
        from_admin = int(self.author.id) in admins[int(server.id)]
        botcommands = self.get_channel_by_name(server, 'bot-commands')
        if botcommands and botcommands[0] != self.channel and not from_admin:
            await self.client.send_message(self.channel, botcommands[0].mention + '?')
            return
        parser = re.search('!set (.+)::(.+)', self.content, re.IGNORECASE)
        if parser and len(parser.group(2)) <= 200 and len(parser.group(1)) > 1 and server.default_role.mention not in parser.group(2) or from_admin:
            try:
                command = smart_command(parser.group(1), parser.group(2), 0, server, self.author.id)
            except VaguePatternError as e:
                await self.client.send_message(self.channel, 'let\'s try making that a little more specific please')
                return

            if not any(command == oldcommand for oldcommand in smart_commands[int(server.id)]) and not len(command.raw_trigger) == 0:
                smart_commands[int(server.id)].append(command)
                new_command = Command(server.id + command.raw_trigger, command.raw_response, command.count, int(server.id), self.author.id)
                self.session.add(new_command)
                self.session.commit()
                await self.client.send_message(self.channel, 'command set')
                return
            elif parser.group(2) == "remove" or parser.group(2) == " remove":
                for oldcommand in smart_commands[int(server.id)]:
                    if oldcommand == command:
                        smart_commands[int(server.id)].remove(oldcommand)
                        self.update_command(oldcommand.raw_trigger, '', 0, server, self.author.id, delete=True)
                        await self.client.send_message(self.channel, 'removed')
                        return
            elif parser.group(2) == "list" or parser.group(2) == " list":
                for oldcommand in smart_commands[int(server.id)]:
                    if oldcommand == command:
                        await self.client.send_message(self.channel, str(oldcommand))
                        return
        await self.client.send_message(self.channel, 'no')

    def get_help(self):
        return "!set <trigger>::<response>\nYou may include the following options:\n[noun],[adj],[adv],[member],[owl],[:reaction:],[count],[comma,separated,choices]"

    def get_usage(self):
        return "!set <trigger>::<response>"

    def update_command(self, triggerkey, response, count, server, author_id, delete=False):
        if (delete):
            self.session.query(Command).filter_by(trigger = server.id + triggerkey).delete()
            self.session.commit()
            return
        new_data = {
                'server_id': server.id,
                'response': response,
                'count': count,
                'author_id': int(author_id)
                }
        self.session.query(Command).filter_by(trigger = server.id + triggerkey).update(new_data)
        self.session.commit()
