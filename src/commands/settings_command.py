
from src.server_settings import server_settings
from src.commands.abstract_command import abstract_command
import time
import discord
import re

TRASH = u"\U0001F5D1"
OPEN_FOLDER = u"\U0001F4C2"
BOT_FACE = u"\U0001F916"
SHIELD = u"\U0001F6E1"
LOCK_KEY = u"\U0001F510"
SWORDS= u"\U00002694"
HAMMER_PICK = u"\U00002692"
HAMMER = u"\U0001F528"

class settings_command(abstract_command):

    def __init__(self):
        super().__init__("settings")

    async def exec_cmd(self, **kwargs):
        session = kwargs['session']
        settings = kwargs['settings']
        self.settings = settings
        if self.author.id not in settings.admins_ids:
            self.client.send_message(self.channel, 'nope, sorry')
            return

        msg = await self.client.send_message(self.channel, embed=await self.get_embed())

        await self.client.add_reaction(msg, "⭐")
        await self.client.add_reaction(msg, TRASH)
        await self.client.add_reaction(msg, OPEN_FOLDER)
        await self.client.add_reaction(msg, BOT_FACE)
        await self.client.add_reaction(msg, SHIELD)
        await self.client.add_reaction(msg, LOCK_KEY)
        await self.client.add_reaction(msg, SWORDS)
        await self.client.add_reaction(msg, HAMMER_PICK)
        await self.client.add_reaction(msg, HAMMER)

        while True:
            react = await self.client.wait_for_reaction(message=msg, user=self.author)
            if react.user == self.client.user: continue
            e = react.reaction.emoji
            print(e)
            if e == '⭐':
                await self.starboard_threshold()
            elif e == TRASH:
                await self.repost_deletes()
            elif e == OPEN_FOLDER:
                await self.manage_emojis()
            elif e == BOT_FACE:
                await self.bot_commands()
            elif e == SHIELD:
                await self.default_role()
            elif e == LOCK_KEY:
                await self.admins()
            elif e == SWORDS:
                await self.roles()
            elif e == HAMMER_PICK:
                await self.gulag_threshold()
            elif e == HAMMER:
                await self.gulag_severity()
            await self.client.edit_message(msg, embed=await self.get_embed())

        return True

    async def starboard_threshold(self):
        await self.client.send_message(self.channel, '⭐ This is the number of reacts a message must get to be starboarded. Enter a number to modify it:')
        msg = await self.client.wait_for_message(author=self.author)
        try:
            self.settings.starboard_threshold = abs(int(msg.content))
            resp = "Threshold set"
        except:
            resp = "Threshold unchanged"
        await self.client.send_message(self.channel, resp)

    async def repost_deletes(self):
        await self.client.send_message(self.channel, '🗑️ If true, deleted messages will be reposted immediately. Enter `true` or `false` to modify it:')
        msg = await self.client.wait_for_message(author=self.author)
        resp = "Setting updated"
        if msg.content in ['1','True','true','yes', 'y']:
            self.settings.repost_del_msg = True
        elif msg.content in ['0', 'False', 'false', 'no']:
            self.settings.repost_del_msg = False
        else:
            resp = "Setting unchanged"

        await self.client.send_message(self.channel, resp)

    async def manage_emojis(self):
        await self.client.send_message(self.channel, '📂 If true, less popular emojis will be cycled in and out as needed, effectively allowing greater than 50 emojis. Enter `true` or `false` to modify it:')
        msg = await self.client.wait_for_message(author=self.author)
        resp = "Setting updated"
        if msg.content in ['1','True','true','yes', 'y']:
            self.settings.manage_emojis = True
        elif msg.content in ['0', 'False', 'false', 'no']:
            self.settings.manage_emojis = False
        else:
            resp = "Setting unchanged"

        await self.client.send_message(self.channel, resp)

    async def bot_commands(self):
        await self.client.send_message(self.channel, "🤖 Some verbose commands are limited to these channels. Mention channels to toggle them:")
        msg = await self.client.wait_for_message(author=self.author)
        bc_channels = self.settings.bot_commands_channels
        new_channels = msg.channel_mentions
        resp = "Setting unchanged"

        for channel in new_channels:
            resp = "Channels updated"
            if channel.id in bc_channels:
                bc_channels.remove(channel.id)
            else:
                bc_channels.append(channel.id)
            self.settings.bot_commands_channels = bc_channels
        await self.client.send_message(self.channel, resp)

    async def default_role(self):
        await self.client.send_message(self.channel, "🛡 New members will be automatically moved into this role. Enter a role id (`!roleids`) to change:")
        def check(msg):
            return msg.content != '!roleids'
        msg = await self.client.wait_for_message(author=self.author, check=check)
        if discord.utils.get(self.server.roles, id=msg.content):
            self.settings.default_role_id = msg.content
            resp = "Default role updated"
        else:
            resp = "Default role unchanged"
        await self.client.send_message(self.channel, resp)

    async def admins(self):
        await self.client.send_message(self.channel, "🔐 These members have access to more bot functions such as `!purge` and setting longer commands. Mention a member to toggle:")
        msg = await self.client.wait_for_message(author=self.author)
        resp = "Admins unchanged"
        if msg.mentions:
            resp = "Admins updated"
            if msg.mentions[0].id in self.settings.admins_ids:
                self.settings.admins_ids.remove(msg.mentions[0].id)
            else:
                self.settings.admins_ids += [msg.mentions[0].id]
        await self.client.send_message(self.channel, resp)

    async def roles(self):
        def check(msg):
            return msg.content != '!roleids'
        await self.client.send_message(self.channel, "⚔ These are the roles that any member can join at will. Enter a list of role ids (`!roleids`) to toggle. Optionally enter a nickname for the role in the format `nickname::roleid` if the role's name is untypable:")
        msg = await self.client.wait_for_message(author=self.author, check=check)
        pattern = re.compile("((?P<nick>\w+)::)?(?P<id>\d{18})")
        new_roles = []
        roles = self.settings.roles_dict
        for match in re.finditer(pattern, msg.content):
            role = discord.utils.get(self.server.roles, id=match.group('id'))
            if match.group('nick') and role:
                new_roles.append((match.group('nick').lower(), role.id))
            elif role:
                new_roles.append((role.name.lower(), role.id))
        resp = "Roles unchanged"
        for role in new_roles:
            resp = "Roles updated"
            if role[1] in roles.values(): # if role already in dict
                if role[0] not in roles:  # in dict with a different nick
                    roles = { k:v for k, v in roles.items() if v != role[1] }
                    roles[role[0]] = role[1]
                else:                     # in dict with the same nick
                    roles = { k:v for k, v in roles.items() if v != role[1] }
            else:                         # new role
                roles[role[0]] = role[1]

        self.settings.roles_dict = roles
        await self.client.send_message(self.channel, resp)

    async def gulag_threshold(self):
        await self.client.send_message(self.channel, HAMMER_PICK + ' This is the number of reacts a gulag vote must get to be pass. Enter a number to modify it:')
        msg = await self.client.wait_for_message(author=self.author)
        try:
            self.settings.gulag_threshold = abs(int(msg.content))
            resp = "Threshold set"
        except:
            resp = "Threshold unchanged"
        await self.client.send_message(self.channel, resp)
    async def gulag_severity(self):
        await self.client.send_message(self.channel, HAMMER + ' This is the number of minutes a member will be confined to gulag. Half again per extra vote. Enter a number to modify it:')
        msg = await self.client.wait_for_message(author=self.author)
        try:
            self.settings.gulag_severity = abs(int(msg.content))
            resp = "Severity set"
        except:
            resp = "Severity unchanged"
        await self.client.send_message(self.channel, resp)

    async def get_embed(self):
        settings = self.settings
        admin_names = list(set([(await self.client.get_user_info(u)).name for u in settings.admins_ids]))
        roles_names = [r.mention for r in [discord.utils.get(self.server.roles, id=i) for i in settings.roles_dict.values()] if r] or ['None']
        bot_commandses = [c.mention for c in [discord.utils.get(self.server.channels, id=i) for i in settings.bot_commands_channels] if c] or ['None']
        default_role = discord.utils.get(self.server.roles, id=settings.default_role_id)

        em = discord.Embed(title="⚙ Settings", description="Select an item for more info, or to change it", colour=0xc1c1ff)
        em.set_author(name='Aut-Bot Server Settings', icon_url='')
        em.add_field(name='⭐ Starboard Threshold ', value='Current value: %d' % settings.starboard_threshold, inline=True)
        em.add_field(name='🗑️ Repost Deleted Messages', value='Current value: %s' % settings.repost_del_msg, inline=True)
        em.add_field(name='📂 Emoji Manager', value='Current value: %s' % settings.manage_emojis, inline=True)
        em.add_field(name='🤖 Bot Commands Channels', value='Current value: %s' % ', '.join(bot_commandses), inline=True)
        em.add_field(name='🛡 Default Role', value='Current value: %s' % (default_role.mention if default_role else 'None'), inline=True)
        em.add_field(name='🔐 Aut-Bot Admins', value='Current value: %s' % ', '.join(admin_names), inline=True)
        em.add_field(name=HAMMER_PICK + ' Gulag Threshold', value='Current value: %d' % settings.gulag_threshold, inline=True)
        em.add_field(name=HAMMER + ' Gulag Severity', value='Current value: %d' % settings.gulag_severity, inline=True)
        em.add_field(name='⚔ Joinable Roles', value='Current value: %s' % ', '.join(roles_names), inline=True)
        return em

    def get_help(self, **kwargs):
        return "Open a settings menu"

    def get_usage(self):
        return ''
