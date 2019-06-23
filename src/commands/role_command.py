from src.list_embed import ListEmbed, dank_embed
from discord.ext import commands
import re
import discord

class Roles(commands.Cog):

    def __init__(self, bot):
        self.bot = bot

    @property
    def guild_settings(self):
        return self.bot.get_cog('GuildSettings')

    @commands.command(aliases=['rank', 'join', 'roles'])
    async def role(self, ctx, *arg):
        '''Assign yourself a role `!roles` to see joinable roles'''
        settings = self.guild_settings.get_guild(ctx.guild)
        roles_dict = settings.roles_dict
        member = ctx.author
        if (len(arg) < 1):
            requested_role = 'list'
        else:
            requested_role = ' '.join(arg)

        if (requested_role == 'list'):
            lembed = ListEmbed('Available Roles', '`!role [role]`', self.bot.user)
            roles = "Available roles:\n"
            for nick, channelid in roles_dict.items():
                role = discord.utils.get(ctx.guild.roles, id=channelid)
                lembed.add(nick, role.mention)
            await ctx.channel.send(embed=lembed.get_embed())

        elif requested_role in roles_dict:
            #filtered = filter(lambda role: role.name == ROLES_DICT[requested_role], member.server.role_hierarchy)
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
            except:
                await ctx.channel.send("Could not add %s to %s." % (ctx.author.mention, requested_role))
            else:
                await ctx.channel.send("%s %s %s %s." % (action, ctx.author.mention, prep, requested_role))
        else:
            await ctx.channel.send("I don't know that role, %s" % ctx.author.mention)

def setup(bot):
    bot.add_cog(Roles(bot))
