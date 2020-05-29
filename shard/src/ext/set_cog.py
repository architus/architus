from discord.ext import commands
from src.auto_response import GuildAutoResponses, TriggerCollisionException, LongResponseException,\
    ShortTriggerException, UserLimitException, UnknownResponseException, DisabledException, PermissionException
from lib.response_grammar.response import ParseError
from lib.reggy.reggy import NotParseable
from src.utils import bot_commands_only
from lib.config import logger

from contextlib import suppress

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
        if not self.bot.settings[msg.channel.guild].responses_enabled:
            return
        resp_msg, response = await self.responses[msg.guild.id].execute(msg)
        if resp_msg is not None:
            self.response_msgs[resp_msg.id] = response

    @commands.Cog.listener()
    async def on_guild_join(self, guild):
        self.responses[guild.id] = GuildAutoResponses(self.bot, guild)

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        msg = react.message
        settings = self.bot.settings[msg.guild]
        if not user.bot and str(react.emoji) == settings.responses_whois_emoji:
            with suppress(KeyError):
                resp = self.response_msgs[msg.id]
                author = msg.channel.guild.get_member(resp.author_id)
                await msg.channel.send(
                    f"{user.mention}, this message came from `{self.response_msgs[msg.id]}`, created by {author}")
                del self.response_msgs[msg.id]

    @commands.command()
    @bot_commands_only
    async def remove(self, ctx, trigger):
        """remove an auto response"""
        settings = self.bot.settings[ctx.guild]
        prefix = re.escape(settings.command_prefix)

        match = re.match(f'{prefix}remove (.+)', ctx.message.content, re.IGNORECASE)
        if match:
            try:
                resp = self.responses[ctx.guild.id].remove(match[1], ctx.author)
            except PermissionException as e:
                member = ctx.guild.get_member(e.author_id)
                whom = f"{member.display_name} or an admin" if member else "an admin"
                await ctx.send(f"❌ please ask {whom} to remove this response")
            except UnknownResponseException:
                await ctx.send("❌ idk what response you want me to remove")
            else:
                await ctx.send(f"✅ `{resp}` _successfully removed_")

    @commands.command()
    @bot_commands_only
    async def set(self, ctx, *args):
        """
        Sets an auto response
        use the syntax 'set trigger::response'
        check out the docs for advanced options:
        https://docs.archit.us/features/auto-responses/
        """
        settings = self.bot.settings[ctx.guild]
        prefix = re.escape(settings.command_prefix)

        match = re.match(f'{prefix}set (.+?)::(.+)', ctx.message.content, re.IGNORECASE)
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
            except ParseError as e:
                await ctx.send(f"❌ unable to parse that response: `{e}`")
            except NotParseable as e:
                await ctx.send(f"❌ unable to parse your trigger: `{e}`")
            except DisabledException as e:
                await ctx.send(f"❌ {e} disabled, you can enable in `{settings.command_prefix}settings responses`")
            except Exception:
                logger.exception("")
                await ctx.send("❌ unknown error 😵")
            else:
                await ctx.send(f"✅ `{resp}` _successfully set_")
        else:
            match = re.match(f'{prefix}set (.+?):(.+)', ctx.message.content, re.IGNORECASE)
            if match:
                await ctx.send(f"❌ **nice brain** use two `::`\n`{prefix}set {match[1]}::{match[2]}`")
            else:
                await ctx.send("❌ use the syntax: `trigger::response`")


def setup(bot):
    bot.add_cog(AutoResponseCog(bot))
