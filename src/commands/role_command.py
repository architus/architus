from src.commands.abstract_command import abstract_command
from src.list_embed import list_embed, dank_embed
import re
import discord

class role_command(abstract_command):

    def __init__(self):
        super().__init__("role", aliases=['rank', 'join', 'roles'])

    async def exec_cmd(self, **kwargs):
        settings = kwargs['settings']
        self.roles_dict = settings.roles_dict
        roles_dict = self.roles_dict
        await self.client.send_typing(self.channel)
        arg = self.content.split(' ')
        member = self.author
        if (len(arg) < 2):
            requested_role = 'list'
        else:
            del arg[0]
            requested_role = ' '.join(arg)

        if (requested_role == 'list'):
            lembed = list_embed('Available Roles', '`!role [role]`', self.client.user)
            roles = "Available roles:\n"
            for nick, channelid in roles_dict.items():
                role = discord.utils.get(self.server.roles, id=channelid)
                lembed.add(nick, role.mention)
            await self.client.send_message(self.channel, embed=lembed.get_embed())
        elif requested_role in roles_dict:
            #filtered = filter(lambda role: role.name == ROLES_DICT[requested_role], member.server.role_hierarchy)
            role = discord.utils.get(self.server.roles, id=roles_dict[requested_role.lower()])
            action = 'Added'
            prep = 'to'
            try:
                if (role in member.roles):
                    await self.client.remove_roles(member, role)
                    action = 'Removed'
                    prep = 'from'
                else:
                    await self.client.add_roles(member, role)
            except:
                await self.client.send_message(self.channel, "Could not add %s to %s." % (self.author.mention, requested_role))
            else:
                await self.client.send_message(self.channel, "%s %s %s %s." % (action, self.author.mention, prep, requested_role))
        else:
            await self.client.send_message(self.channel, "I don't know that role, %s" % self.author.mention)

        return True

    def get_help(self, **kwargs):
        #lembed = list_embed('Available Roles', '`!role [role]`', self.client.user)
        settings = kwargs['settings']
        self.roles_dict = settings.roles_dict
        help_txt = "Available Roles:\n```"
        roles = "Available roles:\n"
        for nick, channelid in self.roles_dict.items():
            role = discord.utils.get(settings.server.roles, id=channelid)
            #lembed.add(nick, role.mention)
            help_txt += '{0:15} {1}'.format('`' + nick + '`', role.mention) + '\n'

        #return lembed.get_embed()
        return help_txt + '```\nUse the nickname in the left column'

    def get_brief(self):
        return "assign yourself a role"

    def get_usage(self):
        return "<role>"
