from src.commands.abstract_command import abstract_command
from src.server_settings import server_settings
from src.models import User, Admin, Command
from discord import ChannelType
from src.smart_command import smart_command, VaguePatternError
import re
import discord

class set_command(abstract_command):

    def __init__(self):
        super().__init__("set", aliases=["remove"])

    async def exec_cmd(self, **kwargs):
        self.session = kwargs['session']
        smart_commands = kwargs['smart_commands']
        admins = kwargs['admins']
        server = self.server
        settings = server_settings(self.session, self.server)
        from_admin = self.author.id in settings.admins_ids
        if settings.bot_commands_channels and self.channel.id not in settings.bot_commands_channels and not from_admin:
            for channelid in settings.bot_commands_channels:
                botcommands = discord.utils.get(self.server.channels, id=channelid, type=ChannelType.text)
                if botcommands:
                    await self.client.send_message(self.channel, botcommands.mention + '?')
                    return
        if self.args[0] == '!remove':
            self.content = '!set %s::remove' % self.content.replace('!remove ', '')
        parser = re.search('!set (.+)::(.+)', self.content, re.IGNORECASE)
        msg = "try actually reading the syntax"
        if parser and len(parser.group(2)) <= 200 and len(parser.group(1)) > 1 and server.default_role.mention not in parser.group(2) or from_admin:
            try:
                command = smart_command(parser.group(1), parser.group(2), 0, server, self.author.id)
            except VaguePatternError as e:
                await self.client.send_message(self.channel, 'let\'s try making that a little more specific please')
                return

            if not any(command == oldcommand for oldcommand in smart_commands[int(server.id)]) and not len(command.raw_trigger) == 0 and command.raw_response not in ['remove', 'author']:
                smart_commands[int(server.id)].append(command)
                new_command = Command(server.id + command.raw_trigger, command.raw_response, command.count, int(server.id), self.author.id)
                self.session.add(new_command)
                self.session.commit()
                msg = 'command set'
            elif parser.group(2) == "remove" or parser.group(2) == " remove":
                msg = 'no command with that trigger'
                for oldcommand in smart_commands[int(server.id)]:
                    if oldcommand == command:
                        smart_commands[int(server.id)].remove(oldcommand)
                        self.update_command(oldcommand.raw_trigger, '', 0, server, self.author.id, delete=True)
                        msg = 'removed `' + oldcommand.raw_trigger + "::" + oldcommand.raw_response + '`'
            elif parser.group(2) == "list" or parser.group(2) == " list":
                for oldcommand in smart_commands[int(server.id)]:
                    if oldcommand == command:
                        msg = str(oldcommand)
                    else:
                        msg = 'no command with that trigger'
            elif parser.group(2) == "author" or parser.group(2) == " author":
                msg = 'no command with that trigger'
                for oldcommand in smart_commands[int(server.id)]:
                    if oldcommand == command:
                        try:
                            usr = await self.client.get_user_info(str(oldcommand.author_id))
                            msg = usr.name + '#' + usr.discriminator
                        except:
                            msg = 'idk'
            else:
                for oldcommand in smart_commands[int(server.id)]:
                    if oldcommand == command:
                        msg = 'Remove `%s` first' % (oldcommand.raw_trigger)
        elif parser and len(parser.group(2)) >= 200:
            msg = 'too long, sorry. ask the owner to set it'
        elif parser and len(parser.group(1)) > 1:
            msg = 'too short'
        await self.client.send_message(self.channel, msg)

        return True

    def get_help(self, **kwargs):
        return "Sets a custom command\nYou may include the following options:\n[noun], [adj], [adv], [member], [owl], [:reaction:], [count], [comma,separated,choices]"
    def get_brief(self):
        return "Set a custom command"

    def get_usage(self):
        return "<trigger>::<response>"

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
