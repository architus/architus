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
        self.response_msgs = {}

    @commands.Cog.listener()
    async def on_ready(self):
        self.responses = {g.id: GuildAutoResponses(self.bot, g) for g in self.bot.guilds}

    @commands.Cog.listener()
    async def on_message(self, msg):
        msg, response = await self.responses[msg.guild.id].execute(msg)
        if msg is not None:
            self.response_msgs[msg.id] = response

    @commands.Cog.listener()
    async def on_guild_join(self, guild):
        self.responses[guild.id] = GuildAutoResponses(self.bot, guild)

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        msg = react.message
        settings = self.bot.settings[msg.guild]
        if str(react.emoji) == settings.responses_whois_emoji:
            resp = self.response_msgs[msg.id]
            if resp:
                author = msg.channel.guild.get_member(resp.author_id)
                await msg.channel.send(
                    f"{user.mention}, this message came from `{self.response_msgs[msg.id]}`, created by {author}")

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
                await ctx.send(f"✅ `{resp}` _successfully removed_")
                return
        await ctx.send("❌ idk what response you want me to remove")

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
                msg = "❌ sorry that trigger collides with the following auto responses:\n"
                msg += '\n'.join([f"`{r}`" for r in e.conflicts[:4]])
                if len(e.conflicts) > 4:
                    msg += f"\n_...{len(e.conflicts) - 4} more not shown_"
                await ctx.send(msg)
            except LongResponseException:
                await ctx.send(f"❌ that response is too long :confused: max length is "
                               f"{settings.responses_trigger_length} characters")
            except ShortTriggerException:
                await ctx.send(
                    f"❌ please make your trigger longer than {settings.responses_trigger_length} characters")
            except UserLimitException:
                await ctx.send(f"❌ looks like you've already used all your auto responses "
                               f"in this server ({settings.responses_limit}), try deleting some")
            else:
                await ctx.send(f"✅ `{resp}` _successfully set_")
        else:
            match = re.search(f'{prefix}set (.+?):(.+)', ctx.message.content, re.IGNORECASE)
            if match:
                await ctx.send(f"❌ **nice brain** use two `::`\n`{prefix}set {match[1]}::{match[2]}`")
            else:
                await ctx.send("❌ use the syntax: `trigger::response`")

            # await ctx.send(f"regex: `{resp.trigger_regex}`")
            # await ctx.send(f"tokens: `{resp.response_ast.stringify()}`")


def setup(bot):
    bot.add_cog(AutoResponseCog(bot))
