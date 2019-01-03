
from src.server_settings import server_settings
from src.commands.abstract_command import abstract_command
import time
import discord
TRASH = u"\U0001F5D1"
OPEN_FOLDER = u"\U0001F4C2"
BOT_FACE = u"\U0001F916"
SHIELD = u"\U0001F6E1"
LOCK_KEY = u"\U0001F510"

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
            await self.client.edit_message(msg, embed=await self.get_embed())











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

    async def starboard_threshold(self):
        await self.client.send_message(self.channel, 'This is the number of reacts a message must get to be starboarded. Enter a number to modify it:')
        msg = await self.client.wait_for_message(author=self.author)
        try:
            self.settings.starboard_threshold = int(msg.content)
            resp = "Threshold set"
        except:
            resp = "Threshold unchanged"
        await self.client.send_message(self.channel, resp)

    async def repost_deletes(self):
        await self.client.send_message(self.channel, 'If true, deleted messages will be reposted immediately. Enter `true` or `false` to modify it:')
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
        await self.client.send_message(self.channel, 'If true, less popular emojis will be cycled in and out as needed, effectively allowing greater than 50 emojis. Enter `true` or `false` to modify it:')
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
        await self.client.send_message(self.channel, "Some verbose commands are limited to these channels. Mention channels to toggle them:")
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
        await self.client.send_message(self.channel, "New members will be automatically moved into this role. Enter a role id (`!roleids`) to change:")
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
        await self.client.send_message(self.channel, "These members have access to more bot functions such as `!purge` and setting longer commands. Mention a member to toggle:")
        msg = await self.client.wait_for_message(author=self.author)
        resp = "Admins unchanged"
        if msg.mentions:
            resp = "Admins updated"
            if msg.mentions[0].id in self.settings.admins_ids:
                self.settings.admins_ids.remove(msg.mentions[0].id)
            else:
                self.settings.admins_ids += [msg.mentions[0].id]
        await self.client.send_message(self.channel, resp)


    async def get_embed(self):
        settings = self.settings
        admin_names = list(set([(await self.client.get_user_info(u)).name for u in settings.admins_ids]))
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
        return em

    def get_help(self):
        return ""

    def get_usage(self):
        return ''
