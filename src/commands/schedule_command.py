import time
import datetime
import pytz
import discord
import dateutil.parser
import re
from unidecode import unidecode
from contextlib import suppress
from discord.ext.commands import Cog
from discord.ext import commands

class ScheduleEvent(object):
    def __init__(self, msg, title, time_str):
        self.msg = msg
        self.title_str = title
        self.parsed_time = time_str
        self.yes = set()
        self.no = set()
        self.maybe = set()

class PollEvent(object):
    def __init__(self, msg, title, options, votes):
        self.msg = msg
        self.title = title
        self.options = options
        self.votes = votes

class EventCog(Cog):

    YES_EMOJI = '✅'
    NO_EMOJI = '❌'
    MAYBE_EMOJI = '🤷'

    ANSWERS = [
            '\N{DIGIT ZERO}\N{COMBINING ENCLOSING KEYCAP}',
            '\N{DIGIT ONE}\N{COMBINING ENCLOSING KEYCAP}',
            '\N{DIGIT TWO}\N{COMBINING ENCLOSING KEYCAP}',
            '\N{DIGIT THREE}\N{COMBINING ENCLOSING KEYCAP}',
            '\N{DIGIT FOUR}\N{COMBINING ENCLOSING KEYCAP}',
            '\N{DIGIT FIVE}\N{COMBINING ENCLOSING KEYCAP}',
            '\N{DIGIT SIX}\N{COMBINING ENCLOSING KEYCAP}',
            '\N{DIGIT SEVEN}\N{COMBINING ENCLOSING KEYCAP}',
            '\N{DIGIT EIGHT}\N{COMBINING ENCLOSING KEYCAP}',
            '\N{DIGIT NINE}\N{COMBINING ENCLOSING KEYCAP}'
    ]

    def __init__(self, bot):
        self.bot = bot
        self.schedule_messages = {}
        self.poll_messages = {}

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        if not user.bot and react.message.id in self.schedule_messages:
            event = self.schedule_messages[react.message.id]
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
            await react.message.edit(
                    content=self.render_schedule_text(event.title_str, event.parsed_time, event.yes, event.no, event.maybe))

        elif not user.bot and react.message.id in self.poll_messages:
            event = self.poll_messages[react.message.id]
            try:
                i = self.ANSWERS.index(str(react.emoji))
            except ValueError:
                return
            event.votes[i].append(user)
            await react.message.edit(
                    content=self.render_poll_text(event.title, event.options, event.votes))


    @commands.Cog.listener()
    async def on_reaction_remove(self, react, user):

        if not user.bot and react.message.id in self.schedule_messages:
            event = self.schedule_messages[react.message.id]
            with suppress(KeyError):
                if self.YES_EMOJI in str(react.emoji):
                    event.yes.remove(user)
                elif self.NO_EMOJI in str(react.emoji):
                    event.no.remove(user)
                elif self.MAYBE_EMOJI in str(react.emoji):
                    event.maybe.remove(user)
            await react.message.edit(
                    content=self.render_schedule_text(event.title_str, event.parsed_time, event.yes, event.no, event.maybe))

        elif not user.bot and react.message.id in self.poll_messages:
            event = self.poll_messages[react.message.id]
            try:
                i = self.ANSWERS.index(str(react.emoji))
            except ValueError:
                return
            event.votes[i].remove(user)
            await react.message.edit(
                    content=self.render_poll_text(event.title, event.options, event.votes))


    async def prompt_date(self, ctx, author):
        await ctx.channel.send("what time?")
        time_msg = await self.bot.wait_for('message', timeout=30, check=lambda m: m.author == author)
        try:
            return dateutil.parser.parse(time_msg.clean_content)
        except:
            await ctx.channel.send("not sure what that means")
            return None

    async def prompt_title(self, ctx, author):
        await ctx.channel.send("what event?")
        title_msg = await self.bot.wait_for('message', timeout=30, check=lambda m: m.author == author)
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
            parsed_time = await self.prompt_date(ctx, ctx.author)
            if not parsed_time:
                return
            parsed_time = tz.localize(parsed_time)
        if len(title) == 0:
            title_str = await self.prompt_title(ctx, ctx.author)
            if not title_str:
                return
        else:
            title_str = ' '.join(title)

        msg = await ctx.channel.send(self.render_schedule_text(title_str, parsed_time, [], [], []))
        await msg.add_reaction(self.YES_EMOJI)
        await msg.add_reaction(self.NO_EMOJI)
        await msg.add_reaction(self.MAYBE_EMOJI)
        self.schedule_messages[msg.id] = ScheduleEvent(msg, title_str, parsed_time)

    @commands.command()
    async def poll(self, ctx, *args):
        '''Starts a poll with some pretty formatting. Supports up to 10 options'''
        pattern = re.compile('!poll (?P<title>(?:\S+[^\s,] )+)(?P<options>.*$)')
        match = pattern.search(unidecode(ctx.message.content))
        if not match: return

        votes = [[] for x in range(10)]
        options = [o.lstrip() for o in match.group('options').split(",")[:10]]
        title = match.group('title').replace('"', '')
        text = self.render_poll_text(title, options, votes)

        msg = await ctx.channel.send(text)
        for i in range(len(options)):
            await msg.add_reaction(self.ANSWERS[i])

        self.poll_messages[msg.id] = PollEvent(msg, title, options, votes)

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

    def render_schedule_text(self, title_str, parsed_time, yes, no, maybe):
        return "**__%s__**\n**Time:** %s\n:white_check_mark: **Yes (%d):** %s\n:x: **No (%d):** %s\n:shrug: **Maybe (%d):** %s" % (
                title_str.strip(),
                parsed_time.strftime("%b %d %I:%M%p %Z"),
                len(yes), ' '.join([u.mention for u in yes]),
                len(no), ' '.join([u.mention for u in no]),
                len(maybe), ' '.join([u.mention for u in maybe])
        )

    def render_poll_text(self, title, options, votes):
        text = f"**__{title.strip()}__**\n"
        i = 0
        for option in options:
            text += "%s **%s (%d)**: %s\n" % (
                    self.ANSWERS[i],
                    option,
                    len(votes[i]), ' '.join([u.mention for u in votes[i]])
            )
            i += 1
        return text

def setup(bot):
    bot.add_cog(EventCog(bot))
