from discord.ext import commands
from src.auto_response import GuildAutoResponses, TriggerCollisionException, LongResponseException,\
    ShortTriggerException, UserLimitException, UnknownResponseException, DisabledException, PermissionException
from lib.response_grammar.response import ParseError
from lib.reggy.reggy import NotParseable
from src.utils import bot_commands_only, doc_url
from lib.config import logger

from contextlib import suppress
from concurrent.futures import ThreadPoolExecutor
from dataclasses import dataclass
from collections import defaultdict
import re


@dataclass
class MsgOvertaken:
    overtaken: bool = False


class AutoResponseCog(commands.Cog, name="Auto Responses"):

    def __init__(self, bot):
        self.bot = bot
        self.responses = {}
        self.response_msgs = {}
        self.react_msgs = {}
        self.executor = ThreadPoolExecutor(max_workers=5)
        self.last_overtaken_ptrs = defaultdict(MsgOvertaken)

    @commands.Cog.listener()
    async def on_ready(self):
        self.responses = {g.id: await GuildAutoResponses.new(self.bot, g, self.executor) for g in self.bot.guilds}
        logger.debug("auto responses initialized")

    @commands.Cog.listener()
    async def on_message(self, msg):
        if not self.bot.settings[msg.channel.guild].responses_enabled:
            return
        self.last_overtaken_ptrs[msg.guild.id].overtaken = True
        with suppress(KeyError):
            self.last_overtaken_ptrs[msg.guild.id] = MsgOvertaken(False)
            resp_msg, response = await self.responses[msg.guild.id].execute(msg, self.last_overtaken_ptrs[msg.guild.id])
            if resp_msg is not None:
                self.response_msgs[resp_msg.id] = response

    @commands.Cog.listener()
    async def on_guild_join(self, guild):
        self.responses[guild.id] = await GuildAutoResponses.new(self.bot, guild, self.executor)

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        msg = react.message
        settings = self.bot.settings[msg.guild]
        if not user.bot and str(react.emoji) == settings.responses_whois_emoji:
            with suppress(KeyError):
                resp = self.response_msgs[msg.id]
                author = msg.channel.guild.get_member(resp.author_id)
                react_msg = await msg.channel.send(
                    f"{user.mention}, this message came from `{self.response_msgs[msg.id]}`, "
                    f"created by {author}\n:x: to remove this auto response")
                # await react_msg.add_reaction("❌")
                self.react_msgs[react_msg.id] = self.response_msgs[msg.id]
                del self.response_msgs[msg.id]
        elif not user.bot and str(react.emoji) == "❌":
            with suppress(KeyError):
                resp = self.react_msgs[msg.id]
                try:
                    await self.responses[resp.guild_id].remove(resp.trigger, user)
                except PermissionException as e:
                    member = msg.guild.get_member(e.author_id)
                    whom = f"{member.display_name} or an admin" if member else "an admin"
                    await msg.channel.send(f"❌ please ask {whom} to remove this response")
                    return
                except UnknownResponseException:
                    await msg.channel.send("❌ This autoresponse has already been removed")
                else:
                    del self.react_msgs[msg.id]
                    await msg.channel.send(f"✅ `{resp}` _successfully removed_")

    @commands.command()
    @bot_commands_only
    @doc_url("https://docs.archit.us/features/auto-responses/#removing-auto-responses")
    async def remove(self, ctx, trigger):
        """remove <trigger>::<response>
        Remove an auto response."""
        settings = self.bot.settings[ctx.guild]
        prefix = re.escape(settings.command_prefix)

        match = re.match(f'^{prefix}remove (.+?)(::.+)?$', ctx.message.content.strip(), re.IGNORECASE)
        if match:
            resp = await self._remove(ctx.guild, match[1], ctx.author)
            if resp:
                await ctx.send(resp)

    async def _remove(self, guild, trigger, author):
        try:
            resp = await self.responses[guild.id].remove(trigger, author)
        except PermissionException as e:
            member = guild.get_member(e.author_id)
            whom = f"{member.display_name} or an admin" if member else "an admin"
            return f"❌ please ask {whom} to remove this response"
        except UnknownResponseException:
            return "❌ idk what response you want me to remove"
        else:
            return f"✅ `{resp}` _successfully removed_"

    @commands.command()
    @bot_commands_only
    @doc_url("https://docs.archit.us/features/auto-responses/#setting-auto-responses")
    async def set(self, ctx):
        """set <trigger>::<response>
        Sets an auto response.
        """
        await self._set(ctx)

    @commands.command()
    @bot_commands_only
    @doc_url("https://docs.archit.us/features/auto-responses/#setting-auto-responses")
    async def setr(self, ctx):
        """set <regex>::<response>
        Sets a regex auto response.
        """
        await self._set(ctx, regex=True)

    @commands.command()
    @bot_commands_only
    @doc_url("https://docs.archit.us/features/auto-responses/#setting-auto-responses")
    async def reply(self, ctx):
        """set <trigger>::<response>
        Sets an auto response that will reply to the trigger message.
        """
        await self._set(ctx, regex=False, reply=True)

    async def _set(self, ctx, regex=False, reply=False):
        settings = self.bot.settings[ctx.guild]
        prefix = re.escape(settings.command_prefix)

        match = re.match(f'{prefix}\\w+ (.+?)::(.+)', ctx.message.content, re.IGNORECASE)
        trigger = f"^{match[1]}$" if regex else match[1]

        if match:
            result = await self.new_response(trigger, match[2], ctx.guild, ctx.author, reply)
            if result:
                await ctx.send(result)
        else:
            match = re.match(f'{prefix}\\w+ ([^\\\\]+?):([^\\\\]+)', ctx.message.content, re.IGNORECASE)
            if match:
                await ctx.send(f"❌ **nice brain** use two `::`\n`{prefix}set {match[1]}::{match[2]}`")
            else:
                await ctx.send("❌ use the syntax: `trigger::response`")

    async def new_response(self, trigger, response, guild, author, reply):
        settings = await self.bot.settings.aio[guild]
        try:
            resp = await self.responses[guild.id].new_response(trigger, response, guild, author, reply)
        except TriggerCollisionException as e:
            msg = "❌ sorry that trigger collides with the following auto responses:\n"
            msg += '\n'.join([f"`{r}`" for r in e.conflicts[:4]])
            if len(e.conflicts) > 4:
                msg += f"\n_...{len(e.conflicts) - 4} more not shown_"
            return msg
        except LongResponseException:
            return f"❌ that response is too long :confused: max length is " \
                   f"{settings.responses_response_length} characters"
        except ShortTriggerException:
            return f"❌ please make your trigger longer than {settings.responses_trigger_length} characters"
        except UserLimitException:
            return f"❌ looks like you've already used all your auto responses " \
                   f"in this server ({settings.responses_limit}), try deleting some"
        except ParseError as e:
            return f"❌ unable to parse that response: `{e}`"
        except NotParseable as e:
            return f"❌ unable to parse your trigger: `{e}`"
        except DisabledException as e:
            return f"❌ {e} disabled, you can enable in `{settings.command_prefix}settings responses`"
        except Exception:
            logger.exception("")
            return "❌ unknown error 😵"
        else:
            return f"✅ `{resp}` _successfully set_"


def setup(bot):
    bot.add_cog(AutoResponseCog(bot))
