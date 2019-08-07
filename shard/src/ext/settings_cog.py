import discord
import re
from discord.ext.commands import Cog
from discord.ext import commands

from src.list_embed import ListEmbed

TRASH = u"\U0001F5D1"
OPEN_FOLDER = u"\U0001F4C2"
BOT_FACE = u"\U0001F916"
SHIELD = u"\U0001F6E1"
LOCK_KEY = u"\U0001F510"
SWORDS = u"\U00002694"
HAMMER_PICK = u"\U00002692"
HAMMER = u"\U0001F528"


class Settings(Cog):
    '''
    Manage server specific aut-bot settings
    '''

    def __init__(self, bot):
        self.bot = bot

    @property
    def guild_settings(self):
        return self.bot.get_cog('GuildSettings')

    @commands.command(hidden=True)
    async def roleids(self, ctx):
        '''Shows the discord id for every role in your server'''
        lem = ListEmbed(ctx.channel.guild.name, '')
        lem.name = "Role IDs"
        for role in ctx.guild.roles:
            lem.add(role.name, role.id)
        await ctx.channel.send(embed=lem.get_embed())

    @commands.command()
    async def settings(self, ctx):
        '''Open an interactive settings dialog'''
        settings = self.guild_settings.get_guild(ctx.guild)
        if ctx.author.id not in settings.admins_ids:
            ctx.channel.send('nope, sorry')
            return True

        msg = await ctx.channel.send(embed=await self.get_embed(ctx))

        await msg.add_reaction("⭐")
        await msg.add_reaction(TRASH)
        await msg.add_reaction(OPEN_FOLDER)
        await msg.add_reaction(BOT_FACE)
        await msg.add_reaction(SHIELD)
        await msg.add_reaction(LOCK_KEY)
        await msg.add_reaction(SWORDS)
        await msg.add_reaction(HAMMER_PICK)
        await msg.add_reaction(HAMMER)

        while True:
            react, user = await self.bot.wait_for(
                'reaction_add', check=lambda r, u: r.message.id == msg.id and u == ctx.author)
            e = react.emoji
            print(e)
            if e == '⭐':
                await self.starboard_threshold(ctx)
            elif e == TRASH:
                await self.repost_deletes(ctx)
            elif e == OPEN_FOLDER:
                await self.manage_emojis(ctx)
            elif e == BOT_FACE:
                await self.bot_commands(ctx)
            elif e == SHIELD:
                await self.default_role(ctx)
            elif e == LOCK_KEY:
                await self.admins(ctx)
            elif e == SWORDS:
                await self.roles(ctx)
            elif e == HAMMER_PICK:
                await self.gulag_threshold(ctx)
            elif e == HAMMER:
                await self.gulag_severity(ctx)
            await msg.edit(embed=await self.get_embed(ctx))

        return True

    async def starboard_threshold(self, ctx):
        await ctx.channel.send(
            '⭐ This is the number of reacts a message must get to be starboarded. Enter a number to modify it:')
        msg = await self.bot.wait_for('message', check=lambda m: m.author == ctx.author)
        self.guild_settings.get_guild(ctx.guild).starboard_threshold = abs(int(msg.content))
        try:
            resp = "Threshold set"
        except Exception:
            resp = "Threshold unchanged"
        await ctx.channel.send(resp)

    async def repost_deletes(self, ctx):
        await ctx.channel.send(
            '🗑️ If true, deleted messages will be reposted immediately. Enter `true` or `false` to modify it:')
        msg = await self.bot.wait_for('message', check=lambda m: m.author == ctx.author)
        resp = "Setting updated"
        settings = self.guild_settings.get_guild(ctx.guild)
        if msg.content in ['1', 'True', 'true', 'yes', 'y']:
            settings.repost_del_msg = True
        elif msg.content in ['0', 'False', 'false', 'no']:
            settings.repost_del_msg = False
        else:
            resp = "Setting unchanged"
        await ctx.channel.send(resp)

    async def manage_emojis(self, ctx):
        await ctx.channel.send('📂 If true, less popular emojis will be cycled in and out as needed, effectively allowing greater than 50 emojis. Enter `true` or `false` to modify it:')
        msg = await self.bot.wait_for('message', check=lambda m: m.author == ctx.author)
        resp = "Setting updated"
        settings = self.guild_settings.get_guild(ctx.guild)
        if msg.content in ['1', 'True', 'true', 'yes', 'y']:
            settings.manage_emojis = True
        elif msg.content in ['0', 'False', 'false', 'no']:
            settings.manage_emojis = False
        else:
            resp = "Setting unchanged"

        await ctx.channel.send(resp)

    async def bot_commands(self, ctx):
        await ctx.channel.send("🤖 Some verbose commands are limited to these channels. Mention channels to toggle them:")
        msg = await self.bot.wait_for('message', check=lambda m: m.author == ctx.author)
        settings = self.guild_settings.get_guild(ctx.guild)
        bc_channels = settings.bot_commands_channels
        new_channels = msg.channel_mentions
        resp = "Setting unchanged"
        settings = self.guild_settings.get_guild(ctx.guild)

        for channel in new_channels:
            resp = "Channels updated"
            if channel.id in bc_channels:
                bc_channels.remove(channel.id)
            else:
                bc_channels.append(channel.id)
            settings.bot_commands_channels = bc_channels
        await ctx.channel.send(resp)

    async def default_role(self, ctx):
        await ctx.channel.send("🛡 New members will be automatically moved into this role. Enter a role id (`!roleids`) to change:")

        def check(msg):
            return msg.content != '!roleids' and msg.author == ctx.author
        msg = await self.bot.wait_for('message', check=check)
        settings = self.guild_settings.get_guild(ctx.guild)
        if discord.utils.get(ctx.guild.roles, id=int(msg.content)):
            settings.default_role_id = msg.content
            resp = "Default role updated"
        else:
            resp = "Default role unchanged"
        await ctx.channel.send(resp)

    async def admins(self, ctx):
        await ctx.channel.send("🔐 These members have access to more bot functions such as `!purge` and setting longer commands. Mention a member to toggle:")
        msg = await self.bot.wait_for('message', check=lambda m: m.author == ctx.author)
        resp = "Admins unchanged"
        settings = self.guild_settings.get_guild(ctx.guild)
        if msg.mentions:
            resp = "Admins updated"
            admin_ids = settings.admin_ids
            if msg.mentions[0].id in settings.admins_ids:
                admin_ids.remove(msg.mentions[0].id)
            else:
                admin_ids.append(msg.mentions[0].id)
            settings.admin_ids = admin_ids
        await ctx.channel.send(resp)

    async def roles(self, ctx):
        def check(msg):
            return msg.content != '!roleids' and msg.author == ctx.author
        await ctx.channel.send("⚔ These are the roles that any member can join at will. Enter a list of role ids (`!roleids`) to toggle. Optionally enter a nickname for the role in the format `nickname::roleid` if the role's name is untypable:")
        msg = await self.bot.wait_for('message', check=check)
        pattern = re.compile(r"((?P<nick>\w+)::)?(?P<id>\d{18})")
        new_roles = []
        settings = self.guild_settings.get_guild(ctx.guild)
        roles = settings.roles_dict
        for match in re.finditer(pattern, msg.content):
            role = discord.utils.get(ctx.guild.roles, id=int(match.group('id')))
            if match.group('nick') and role:
                new_roles.append((match.group('nick').lower(), role.id))
            elif role:
                new_roles.append((role.name.lower(), role.id))
        resp = "Roles unchanged"
        for role in new_roles:
            resp = "Roles updated"
            if role[1] in roles.values():  # if role already in dict
                if role[0] not in roles:  # in dict with a different nick
                    roles = {k: v for k, v in roles.items() if v != role[1]}
                    roles[role[0]] = role[1]
                else:                     # in dict with the same nick
                    roles = {k: v for k, v in roles.items() if v != role[1]}
            else:                         # new role
                roles[role[0]] = role[1]

        settings.roles_dict = roles
        await ctx.channel.send(resp)

    async def gulag_threshold(self, ctx):
        await ctx.channel.send(HAMMER_PICK + ' This is the number of reacts a gulag vote must get to be pass. Enter a number to modify it:')
        msg = await self.bot.wait_for('message', check=lambda m: m.author == ctx.author)
        settings = self.guild_settings.get_guild(ctx.guild)
        try:
            settings.gulag_threshold = abs(int(msg.content))
            resp = "Threshold set"
        except Exception:
            resp = "Threshold unchanged"
        await ctx.channel.send(resp)

    async def gulag_severity(self, ctx):
        await ctx.channel.send(HAMMER + ' This is the number of minutes a member will be confined to gulag. Half again per extra vote. Enter a number to modify it:')
        settings = self.guild_settings.get_guild(ctx.guild)
        msg = await self.bot.wait_for('message', check=lambda m: m.author == ctx.author)
        try:
            settings.gulag_severity = abs(int(msg.content))
            resp = "Severity set"
        except ValueError:
            resp = "Severity unchanged"
        await ctx.channel.send(resp)

    async def get_embed(self, ctx):
        settings = self.guild_settings.get_guild(ctx.guild)
        admin_names = list(set([(await self.bot.fetch_user(u)).name for u in settings.admins_ids]))
        roles_names = [r.mention for r in [discord.utils.get(ctx.guild.roles, id=i) for i in settings.roles_dict.values()] if r] or ['None']
        bot_commandses = [c.mention for c in [discord.utils.get(ctx.guild.channels, id=i) for i in settings.bot_commands_channels] if c] or ['None']
        default_role = discord.utils.get(ctx.guild.roles, id=settings.default_role_id)

        em = discord.Embed(title="⚙ Settings", description="Select an item for more info, or to change it", colour=0xc1c1ff)
        em.set_author(name='Architus Server Settings', icon_url='')
        em.add_field(name='⭐ Starboard Threshold ', value='Current value: %d' % settings.starboard_threshold, inline=True)
        em.add_field(name='🗑️ Repost Deleted Messages', value='Current value: %s' % settings.repost_del_msg, inline=True)
        em.add_field(name='📂 Emoji Manager', value='Current value: %s' % settings.manage_emojis, inline=True)
        em.add_field(name='🤖 Bot Commands Channels', value='Current value: %s' % ', '.join(bot_commandses), inline=True)
        em.add_field(name='🛡 Default Role', value='Current value: %s' % (default_role.mention if default_role else 'None'), inline=True)
        em.add_field(name='🔐 Architus Admins', value='Current value: %s' % ', '.join(admin_names), inline=True)
        em.add_field(name=HAMMER_PICK + ' Gulag Threshold', value='Current value: %d' % settings.gulag_threshold, inline=True)
        em.add_field(name=HAMMER + ' Gulag Severity', value='Current value: %d' % settings.gulag_severity, inline=True)
        em.add_field(name='⚔ Joinable Roles', value='Current value: %s' % ', '.join(roles_names), inline=True)
        return em


def setup(bot):
    bot.add_cog(Settings(bot))
