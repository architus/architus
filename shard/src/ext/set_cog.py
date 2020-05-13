from discord.ext import commands
from src.auto_response import GuildAutoResponses
from src.utils import bot_commands_only
from lib.config import logger

import re


class AutoResponseCog(commands.Cog, name="Auto Responses"):

    def __init__(self, bot):
        self.bot = bot
        self.responses = {}

    @commands.Cog.listener()
    async def on_ready(self):
        self.responses = {g.id: GuildAutoResponses(self.bot, g) for g in self.bot.guilds}

    @commands.Cog.listener()
    async def on_message(self, msg):
        await self.responses[msg.guild.id].execute(msg)

    @commands.Cog.listener()
    async def on_guild_join(self, guild):
        self.responses[guild.id] = GuildAutoResponses(self.bot, guild)

    @commands.command()
    @bot_commands_only
    async def remove(self, ctx, trigger):
        pass

    @commands.command()
    @bot_commands_only
    async def set(self, ctx, *args):
        '''
        Sets a custom command
        You may include the following options:
        [noun], [adj], [adv], [member], [owl], [:reaction:], [count], [comma,separated,choices]
        '''
        settings = self.bot.settings[ctx.guild]
        prefix = settings.command_prefix

        match = re.search(f'{prefix}set (.+?)::(.+)', ctx.message.content, re.IGNORECASE)
        if match:
            resp = self.responses[ctx.guild.id].new(match[1], match[2], ctx.guild, ctx.author)

            await ctx.send(f"`{resp}`")
            await ctx.send(f"id: `{resp.id}`")
            await ctx.send(f"regex: `{resp.trigger_regex}`")
            await ctx.send(f"punc: `{resp.trigger_punctuation}`")
            await ctx.send(f"tokens: `{resp.response_ast.stringify()}`")


def setup(bot):
    bot.add_cog(AutoResponseCog(bot))
