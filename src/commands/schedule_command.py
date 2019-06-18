from src.commands.abstract_command import abstract_command
import time
import datetime
import pytz
import discord
import dateutil.parser
from contextlib import suppress
from discord.ext.commands import Cog
from discord.ext import commands

class schedule_command(abstract_command):

    YES_EMOJI = '✅'
    NO_EMOJI = '❌'
    MAYBE_EMOJI = '🤷'

    def __init__(self):
        super().__init__("scheduleold")

    async def exec_cmd(self, **kwargs):
        # event bot's id
        if self.server.get_member(476042677440479252):
            print("not scheduling cause event bot exists")
            return
        reaction_callbacks = kwargs['reaction_callbacks']
        region = self.server.region
        print(region)
        tz = pytz.timezone(self.get_timezone(region))
        ct = datetime.datetime.now(tz=tz)
        del self.args[0]
        title = []
        self.parsed_time = None
        for i in range(len(self.args)):
            with suppress(ValueError):
                print(' '.join(self.args))
                self.parsed_time = dateutil.parser.parse(' '.join(self.args))
                self.parsed_time = tz.localize(self.parsed_time)
                break
            title.append(self.args[0])
            del self.args[0]

        if not self.parsed_time:
            self.parsed_time = await self.prompt_date(self.author)
            if not self.parsed_time: return
            self.parsed_time = tz.localize(self.parsed_time)
        if len(title) == 0:
            self.title_str = await self.prompt_title(self.author)
            if not title_str: return
        else:
            self.title_str = ' '.join(title)

        self.yes = set()
        self.no = set()
        self.maybe = set()
        self.msg = await self.channel.send(self.render_text(self.title_str, self.parsed_time, self.yes, self.no, self.maybe))
        await self.msg.add_reaction(self.YES_EMOJI)
        await self.msg.add_reaction(self.NO_EMOJI)
        await self.msg.add_reaction(self.MAYBE_EMOJI)
        reaction_callbacks[self.msg.id] = (self.on_react_add, self.on_react_remove)
        return True

    async def on_react_add(self, react, user):
        if str(react.emoji) in [self.YES_EMOJI, self.NO_EMOJI, self.MAYBE_EMOJI]:
            with suppress(KeyError):
                self.yes.remove(user)
            with suppress(KeyError):
                self.no.remove(user)
            with suppress(KeyError):
                self.maybe.remove(user)

        if self.YES_EMOJI in str(react.emoji):
            self.yes.add(user)
        elif self.NO_EMOJI in str(react.emoji):
            self.no.add(user)
        elif self.MAYBE_EMOJI in str(react.emoji):
            self.maybe.add(user)
        await self.msg.edit(content=self.render_text(self.title_str, self.parsed_time, self.yes, self.no, self.maybe))

    async def on_react_remove(self, react, user):
        with suppress(KeyError):
            if self.YES_EMOJI in str(react.emoji):
                self.yes.remove(user)
            elif self.NO_EMOJI in str(react.emoji):
                self.no.remove(user)
            elif self.MAYBE_EMOJI in str(react.emoji):
                self.maybe.remove(user)
        await self.msg.edit(content=self.render_text(self.title_str, self.parsed_time, self.yes, self.no, self.maybe))

    def render_text(self, title_str, parsed_time, yes, no, maybe):
        return "__**%s**__\n**Time: **%s\n:white_check_mark: **Yes (%d): %s**\n:x: **No (%d): %s**\n:shrug: **Maybe (%d): %s**" % (title_str, parsed_time.strftime("%b %d %I:%M%p %Z"), len(yes), ' '.join([u.mention for u in yes]), len(no), ' '.join([u.mention for u in no]), len(maybe), ' '.join([u.mention for u in maybe]))

    async def prompt_date(self, author):
        await self.channel.send("what time?")
        time_msg = await self.client.wait_for_message(timeout=30, author=author)
        try:
            return dateutil.parser.parse(time_msg.clean_content)
        except:
            await self.channel.send("not sure what that means")
            return None

    async def prompt_title(self, author):
        await self.channel.send("what event?")
        title_msg = await self.client.wait_for_message(timeout=30, author=author)
        return title_msg.clean_content or None

    def get_help(self, **kwargs):
        return "Start an event poll with pretty formatting. Knows the difference between daylight and standard time."

    def get_brief(self):
        return "Start an event poll with pretty formatting"

    def get_usage(self):
        return "<title> [date]"

    def get_timezone(self, region):
        region = str(region)
        if region == 'us-south' or region == 'us-east':
            return 'America/New_York'
        elif region == 'us-central':
            return 'America/Chicago'
        elif region == 'us-west':
            return 'America/Los_Angeles'
        else:
            return 'Etc/UTC'

class Schedule(Cog):

    YES_EMOJI = '✅'
    NO_EMOJI = '❌'
    MAYBE_EMOJI = '🤷'

    def __init__(self, bot):
        self.bot = bot

    @commands.Cog.listener()
    async def on_reaction_add(react, user):
        if str(react.emoji) in [self.YES_EMOJI, self.NO_EMOJI, self.MAYBE_EMOJI]:
            with suppress(KeyError):
                self.yes.remove(user)
            with suppress(KeyError):
                self.no.remove(user)
            with suppress(KeyError):
                self.maybe.remove(user)

        if self.YES_EMOJI in str(react.emoji):
            self.yes.add(user)
        elif self.NO_EMOJI in str(react.emoji):
            self.no.add(user)
        elif self.MAYBE_EMOJI in str(react.emoji):
            self.maybe.add(user)
        await self.msg.edit(content=self.render_text(self.title_str, self.parsed_time, self.yes, self.no, self.maybe))

    @commands.Cog.listener()
    async def on_reaction_remove(react, user):
        with suppress(KeyError):
            if self.YES_EMOJI in str(react.emoji):
                self.yes.remove(user)
            elif self.NO_EMOJI in str(react.emoji):
                self.no.remove(user)
            elif self.MAYBE_EMOJI in str(react.emoji):
                self.maybe.remove(user)
        await self.msg.edit(content=self.render_text(self.title_str, self.parsed_time, self.yes, self.no, self.maybe))

    async def prompt_date(self, author):
        await self.channel.send("what time?")
        time_msg = await self.client.wait_for_message(timeout=30, author=author)
        try:
            return dateutil.parser.parse(time_msg.clean_content)
        except:
            await self.channel.send("not sure what that means")
            return None

    async def prompt_title(self, author):
        await self.channel.send("what event?")
        title_msg = await self.client.wait_for_message(timeout=30, author=author)
        return title_msg.clean_content or None

    @commands.command()
    async def schedule(self, ctx, *argst):
        args = list(argst)
        print(args)
        # event bot's id
        if ctx.guild.get_member(476042677440479252):
            print("not scheduling cause event bot exists")
            return
        region = ctx.guild.region
        print(region)
        tz = pytz.timezone(self.get_timezone(region))
        ct = datetime.datetime.now(tz=tz)
        title = []
        self.parsed_time = None
        for i in range(len(args)):
            with suppress(ValueError):
                print(' '.join(args))
                self.parsed_time = dateutil.parser.parse(' '.join(args))
                self.parsed_time = tz.localize(self.parsed_time)
                break
            title.append(args[0])
            del args[0]

        if not self.parsed_time:
            self.parsed_time = await self.prompt_date(ctx.author)
            if not self.parsed_time: return
            self.parsed_time = tz.localize(self.parsed_time)
        if len(title) == 0:
            self.title_str = await self.prompt_title(ctx.author)
            if not title_str: return
        else:
            self.title_str = ' '.join(title)

        self.yes = set()
        self.no = set()
        self.maybe = set()
        self.msg = await ctx.channel.send(self.render_text(self.title_str, self.parsed_time, self.yes, self.no, self.maybe))
        await self.msg.add_reaction(self.YES_EMOJI)
        await self.msg.add_reaction(self.NO_EMOJI)
        await self.msg.add_reaction(self.MAYBE_EMOJI)

    def get_timezone(self, region):
        region = str(region)
        if region == 'us-south' or region == 'us-east':
            return 'America/New_York'
        elif region == 'us-central':
            return 'America/Chicago'
        elif region == 'us-west':
            return 'America/Los_Angeles'
        else:
            return 'Etc/UTC'

    def render_text(self, title_str, parsed_time, yes, no, maybe):
        return "__**%s**__\n**Time: **%s\n:white_check_mark: **Yes (%d): %s**\n:x: **No (%d): %s**\n:shrug: **Maybe (%d): %s**" % (title_str, parsed_time.strftime("%b %d %I:%M%p %Z"), len(yes), ' '.join([u.mention for u in yes]), len(no), ' '.join([u.mention for u in no]), len(maybe), ' '.join([u.mention for u in maybe]))
def setup(bot):
    bot.add_cog(Schedule(bot))
