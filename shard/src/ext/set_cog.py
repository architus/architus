from discord.ext import commands
from src.auto_response import GuildAutoResponses, TriggerCollisionException, LongResponseException,\
    ShortTriggerException, UserLimitException, UnknownResponseException
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
        settings = self.bot.settings[ctx.guild]
        prefix = settings.command_prefix

        match = re.search(f'{prefix}remove (.+)', ctx.message.content, re.IGNORECASE)
        if match:
            try:
                logger.debug(match[1])
                resp = self.responses[ctx.guild.id].remove(match[1])
            except UnknownResponseException:
                pass
            else:
                await ctx.send(f"removed `{resp}`")
                return
        await ctx.send("idk what response you want me to remove")

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
            try:
                resp = self.responses[ctx.guild.id].new(match[1], match[2], ctx.guild, ctx.author)
            except TriggerCollisionException as e:
                await ctx.send(f"sorry that trigger collides with these other responses: " + '\n'.join([str(r) for r in e.conflicts]))
            except LongResponseException:
                await ctx.send(f"that response is too long :confused:")
            except ShortTriggerException:
                await ctx.send(f"please make your trigger longer")
            except UserLimitException:
                await ctx.send(f"looks like you've already used all your auto responses in this server, try deleting some")
            else:
                await ctx.send(f"autoresponse: `{resp}` succesfully set")
                logger.debug("`{resp.response_ast.stringify()}`")
        else:
            await ctx.send("use the syntax: `trigger::response`")

            # await ctx.send(f"regex: `{resp.trigger_regex}`")
            # await ctx.send(f"tokens: `{resp.response_ast.stringify()}`")


def setup(bot):
    bot.add_cog(AutoResponseCog(bot))
