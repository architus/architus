import time
import datetime
import pytz
import discord
import dateutil.parser
from contextlib import suppress
from discord.ext.commands import Cog
from discord.ext import commands

class Event(object):
    def __init__(self, msg, title, time_str):
        self.msg = msg
        self.title_str = title
        self.parsed_time = time_str
        self.yes = set()
        self.no = set()
        self.maybe = set()

class Schedule(Cog):

    YES_EMOJI = '✅'
    NO_EMOJI = '❌'
    MAYBE_EMOJI = '🤷'

    def __init__(self, bot):
        self.bot = bot
        self.events = {}

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        if user == self.bot.user:
            return
        try:
            event = self.events[react.message.id]
        except KeyError:
            return
        if str(react.emoji) in [self.YES_EMOJI, self.NO_EMOJI, self.MAYBE_EMOJI]:
            with suppress(KeyError):
                event.yes.remove(user)
            with suppress(KeyError):
                event.no.remove(user)
            with suppress(KeyError):
                event.maybe.remove(user)

        if self.YES_EMOJI in str(react.emoji):
            event.yes.add(user)
        elif self.NO_EMOJI in str(react.emoji):
            event.no.add(user)
        elif self.MAYBE_EMOJI in str(react.emoji):
            event.maybe.add(user)
        await react.message.edit(content=self.render_text(event.title_str, event.parsed_time, event.yes, event.no, event.maybe))

    @commands.Cog.listener()
    async def on_reaction_remove(self, react, user):
        if user == self.bot.user:
            return
        try:
            event = self.events[react.message.id]
        except KeyError:
            return
        with suppress(KeyError):
            if self.YES_EMOJI in str(react.emoji):
                event.yes.remove(user)
            elif self.NO_EMOJI in str(react.emoji):
                event.no.remove(user)
            elif self.MAYBE_EMOJI in str(react.emoji):
                event.maybe.remove(user)
        await react.message.edit(content=self.render_text(event.title_str, event.parsed_time, event.yes, event.no, event.maybe))

    async def prompt_date(self, author):
        await self.channel.send("what time?")
        time_msg = await self.client.wait_for('message', timeout=30, check=lambda m: m.author == author)
        try:
            return dateutil.parser.parse(time_msg.clean_content)
        except:
            await self.channel.send("not sure what that means")
            return None

    async def prompt_title(self, author):
        await self.channel.send("what event?")
        title_msg = await self.client.wait_for('message', timeout=30, check=lambda m: m.author == author)
        return title_msg.clean_content or None

    @commands.command()
    async def schedule(self, ctx, *argst):
        args = list(argst)
        # event bot's id
        if ctx.guild.get_member(476042677440479252):
            print("not scheduling cause event bot exists")
            return
        region = ctx.guild.region
        tz = pytz.timezone(self.get_timezone(region))
        ct = datetime.datetime.now(tz=tz)
        title = []
        parsed_time = None
        for i in range(len(args)):
            with suppress(ValueError):
                parsed_time = dateutil.parser.parse(' '.join(args))
                parsed_time = tz.localize(parsed_time)
                break
            title.append(args[0])
            del args[0]

        if not parsed_time:
            parsed_time = await self.prompt_date(ctx.author)
            if not self.parsed_time: return
            parsed_time = tz.localize(parsed_time)
        if len(title) == 0:
            title_str = await self.prompt_title(ctx.author)
            if not title_str:
                return
        else:
            title_str = ' '.join(title)

        msg = await ctx.channel.send(self.render_text(title_str, parsed_time, [], [], []))
        await msg.add_reaction(self.YES_EMOJI)
        await msg.add_reaction(self.NO_EMOJI)
        await msg.add_reaction(self.MAYBE_EMOJI)
        self.events[msg.id] = Event(msg, title_str, parsed_time)

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
