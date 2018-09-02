from src.commands.abstract_command import abstract_command
from src.list_embed import list_embed, dank_embed
import re
import discord

class role_command(abstract_command):

    def __init__(self):
        super().__init__("role", aliases=['rank', 'join'])

    async def exec_cmd(self, **kwargs):
        ROLES_DICT = kwargs['ROLES_DICT']
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
            for roletag, rolename in ROLES_DICT.items():
                lembed.add(rolename, roletag)
            await self.client.send_message(self.channel, embed=lembed.get_embed())
        elif (requested_role.lower() in (name.lower() for name in ROLES_DICT)):
            filtered = filter(lambda role: role.name == ROLES_DICT[requested_role], member.server.role_hierarchy)
            action = 'Added'
            prep = 'to'
            try:
                role = next(filtered)
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

    def get_help(self):
        lembed = list_embed('Available Roles', '`!role [role]`', self.client.user)
        roles = "Available roles:\n"
        for roletag, rolename in ROLES_DICT.items():
            lembed.add(rolename, roletag)
        return lembed.get_embed()

    def get_usage(self):
        return "!role <role> - give yourself a role"
