from src.list_embed import ListEmbed
from discord.ext import commands
import discord
from contextlib import suppress
from functools import reduce

from lib.config import logger
from lib.aiomodels import Roles as TbRoles
from src.utils import doc_url


class Roles(commands.Cog):

    def __init__(self, bot):
        self.bot = bot
        self.tb_roles = TbRoles(self.bot.asyncpg_wrapper)
        self.role_messages = []

    @commands.Cog.listener()
    async def on_ready(self):
        for guild in self.bot.guilds:
            for r in (await self.tb_roles.select_by_guild(guild.id)):
                self.role_messages.append(r['message_id'])

    async def setup_roles(self, guild, channel, roles):
        roles_str = ""
        for emoji, role in roles.items():
            roles_str += f'{emoji}➧{role.mention}'
        embed = discord.Embed(title="Role Select", description=roles_str)
        msg = await channel.send(embed=embed, allowed_mentions=discord.AllowedMentions.none())
        self.role_messages.append(msg.id)

        await self.tb_roles.delete_by_guild_id(guild.id)
        errors = []
        for emoji, role in roles.items():
            try:
                await msg.add_reaction(emoji)
                await self.tb_roles.insert(
                    {'guild_id': guild.id, 'role_id': role.id, 'message_id': msg.id, 'emoji': emoji})
            except discord.NotFound:
                errors.append(emoji)
        if len(errors) == 0:
            return f"*Successfully registered {len(roles)} roles*"
        else:
            return f"Unknown emoji: {', '.join(errors)}"

    @commands.Cog.listener()
    async def on_raw_reaction_add(self, payload):
        await self.handle_react(payload, True)

    @commands.Cog.listener()
    async def on_raw_reaction_remove(self, payload):
        await self.handle_react(payload, False)

    async def handle_react(self, payload, react_add):
        if payload.message_id not in self.role_messages:
            return
        guild = self.bot.get_guild(payload.guild_id)
        record = await self.tb_roles.select_by_id({'emoji': str(payload.emoji), 'message_id': int(payload.message_id)})
        if record is None:
            return
        member = guild.get_member(int(payload.user_id))
        role = guild.get_role(record['role_id'])
        try:
            if react_add:
                await member.add_roles(role)
            else:
                await member.remove_roles(role)
        except Exception:
            logger.warning(f"Error adding {member} to {role}")

    @commands.Cog.listener()
    async def on_member_join(self, member):
        logger.info("%s joined guild: %s" % (member.name, member.guild.name))
        settings = self.bot.settings[member.guild]
        try:
            default_role = discord.utils.get(member.guild.roles, id=settings.default_role_id)
            if default_role is not None:
                await member.add_roles(default_role)
        except AttributeError:
            pass
        except Exception:
            logger.exception("could not add %s to %s" % (member.display_name, 'default role'))

    @commands.command(aliases=['rank', 'join', 'roles'])
    @doc_url("https://docs.archit.us/features/role-manager")
    async def role(self, ctx, *arg):
        '''role [role to join]
        List available roles to join or join a role.'''
        settings = self.bot.settings[ctx.guild]
        roles_dict = settings.roles_dict
        member = ctx.author
        if (len(arg) < 1):
            requested_role = 'list'
        else:
            requested_role = ' '.join(arg)

        if (requested_role == 'list'):
            lembed = ListEmbed('Available Roles', f'`{settings.command_prefix}role [role]`', self.bot.user)
            for nick, channelid in roles_dict.items():
                role = discord.utils.get(ctx.guild.roles, id=channelid)
                with suppress(AttributeError):
                    lembed.add(nick, role.mention)
            await ctx.channel.send(embed=lembed.get_embed())

        elif requested_role in roles_dict:
            # filtered = filter(lambda role: role.name == ROLES_DICT[requested_role], member.server.role_hierarchy)
            role = discord.utils.get(ctx.guild.roles, id=roles_dict[requested_role.lower()])
            action = 'Added'
            prep = 'to'
            try:
                if (role in member.roles):
                    await member.remove_roles(role, reason="User requested not role")
                    action = 'Removed'
                    prep = 'from'
                else:
                    await member.add_roles(role, reason="User request role")
            except Exception:
                await ctx.channel.send("Could not add %s to %s." % (ctx.author.mention, requested_role))
            else:
                await ctx.channel.send("%s %s %s %s." % (action, ctx.author.mention, prep, requested_role))
        else:
            await ctx.channel.send("I don't know that role, %s" % ctx.author.mention)


def setup(bot):
    bot.add_cog(Roles(bot))
