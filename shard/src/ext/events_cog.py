import pytz
import dateutil.parser
import re
from unidecode import unidecode
from contextlib import suppress
from discord.ext.commands import Cog
from discord.ext import commands
from enum import Enum
import datetime
import json

from lib.aiomodels import TbReactEvents
from lib.config import logger


class ReactionEvent(Enum):
    poll = 0,
    schedule = 1


class EventCog(Cog, name="Events"):
    '''
    Special messages that track the number of participants
    '''

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
        self.tb_react_events = TbReactEvents(self.bot.asyncpg_wrapper)

    @commands.Cog.listener()
    async def on_raw_reaction_add(self, payload):
        channel = await self.bot.fetch_channel(payload.channel_id)
        message = await channel.fetch_message(payload.message_id)
        user = payload.member
        emoji = payload.emoji
        if user.bot: 
            return

        react_event = await self.tb_react_events.get_by_id(message.id, message.guild.id)
        if react_event is None:
            return

        react_event_payload = json.loads(react_event.payload)
        print(react_event_payload)
        if react_event.command == ReactionEvent.schedule:
            self.schedule_react_add(emoji, user, message, react_event_payload)
            return
        elif react_event.command == ReactionEvent.poll:
            self.poll_react_add(emoji, user, message, react_event_payload)
            return

    async def schedule_react_add(self, emoji, user, message, payload):
        reactions = self.message.reactions
        yes = [r.user for r in reactions if str(r.emoji) == self.YES_EMOJI]
        no = [r.user for r in reactions if str(r.emoji) == self.NO_EMOJI]
        maybe = [r.user for r in reactions if str(r.emoji) == self.MAYBE_EMOJI]
        with suppress(KeyError):
            yes.remove(user)
        with suppress(KeyError):
            no.remove(user)
        with suppress(KeyError):
            maybe.remove(user)
        for r in reactions:
            if r.emoji != emoji:
                await r.remove(user)

        if self.YES_EMOJI in str(emoji):
            yes.add(user)
        elif self.NO_EMOJI in str(emoji):
            no.add(user)
        elif self.MAYBE_EMOJI in str(emoji):
            maybe.add(user)
        await message.edit(
            content=self.render_schedule_text(payload.title_str, payload.parsed_time, yes, no, maybe))

    async def poll_react_add(self, emoji, user, message, payload):
        votes = {}
        for r in message.reactions:
            with suppress(ValueError):
                i = self.ANSWERS.index(str(r.emoji))
                votes[i] = [u for u in await r.users().flatten() if u != self.bot.user]
                if payload.exclusive:
                    if i != self.ANSWERS.index(str(emoji)):
                        votes[i].remove(user)
                        await r.remove(user)

        await message.edit(
            content=self.render_poll_text(payload.title, payload.options, votes))

    @commands.Cog.listener()
    async def on_raw_reaction_remove(self, payload):
        message = await self.bot.get_message(payload.channel_id, payload.message_id)
        user = await self.bot.get_member(payload.user_id)
        emoji = payload.emoji
        if self.bot.id != message.author.id or user.bot: 
            return

        react_event = await self.tb_react_events.get_by_id(message.id, message.guild.id)
        if react_event is None:
            return

        react_event_payload = json.loads(react_event.payload)
        if react_event.command == ReactionEvent.poll:
            await self.poll_react_remove(emoji, user, message, react_event_payload)
            return

    async def poll_react_remove(self, emoji, user, message, payload):
        votes = {}
        if payload.exclusive:
            with suppress(ValueError):
                i = self.ANSWERS.index(str(emoji))
                votes[i].remove(user)
                await message.edit(
                    content=self.render_poll_text(payload.title, payload.options, votes))

    async def prompt_date(self, ctx, author):
        await ctx.channel.send("what time?")
        time_msg = await self.bot.wait_for('message', timeout=30, check=lambda m: m.author == author)
        try:
            return dateutil.parser.parse(time_msg.clean_content)
        except Exception:
            await ctx.channel.send("not sure what that means")
            return None

    async def prompt_title(self, ctx, author):
        await ctx.channel.send("what event?")
        title_msg = await self.bot.wait_for('message', timeout=30, check=lambda m: m.author == author)
        return title_msg.clean_content or None

    @commands.command()
    async def schedule(self, ctx, *argst):
        '''
        Start an event poll.
        Timezone is based on your servers voice zone.
        '''
        args = list(argst)
        logger.debug(args)
        # event bot's id
        if ctx.guild.get_member(476042677440479252):
            logger.warning("not scheduling cause event bot exists")
            return
        region = ctx.guild.region
        tz = pytz.timezone(self.get_timezone(region))
        # ct = datetime.datetime.now(tz=tz)
        title = []
        parsed_time = None
        for i in range(len(args)):
            with suppress(ValueError):
                parsed_time = dateutil.parser.parse(' '.join(args))
                # parsed_time = tz.localize(parsed_time)
                break
            with suppress(ValueError):
                parsed_time = dateutil.parser.parse(' '.join(args[:-1]))
                # parsed_time = tz.localize(parsed_time)
                break
            title.append(args[0])
            del args[0]
        else:
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

        event_id = ReactionEvent.schedule
        expires = datetime.datetime.now() + datetime.timedelta(days=1)
        payload = {
            'title_str': title_str,
            'parsed_time': parsed_time
        }
        await self.tb_react_events.insert(msg.id, msg.guild.id, msg.channel.id, event_id, json.dumps(payload), expires)

    @commands.command()
    async def poll(self, ctx, *args):
        '''
        Starts a poll with some pretty formatting
        Allows more than one response per user
        Surround title in quotes to include spaces
        Supports up to 10 options
        '''
        pattern = re.compile(r'.poll (?P<title>(?:(?:".*")|(?:.*?)+)) (?P<options>.*$)')
        match = pattern.search(unidecode(ctx.message.content))
        await self.register_poll(ctx, match, False)

    @commands.command()
    async def xpoll(self, ctx, *args):
        '''
        Starts a exclusive poll with some pretty formatting
        Limited to one response per user
        Surround title in quotes to include spaces
        Supports up to 10 options
        '''
        pattern = re.compile(r'.poll (?P<title>(?:(?:".*")|(?:.*?)+)) (?P<options>.*$)')
        match = pattern.search(unidecode(ctx.message.content))
        await self.register_poll(ctx, match, True)

    async def register_poll(self, ctx, match, exclusive: bool):
        if not match:
            await ctx.send("Sorry, I couldn't parse that nonsense.")
            return

        votes = [[] for x in range(10)]
        options = [o.lstrip() for o in match.group('options').split(",")[:10]]
        title = match.group('title').replace('"', '')
        text = self.render_poll_text(title, options, votes)

        msg = await ctx.channel.send(text)
        for i in range(len(options)):
            await msg.add_reaction(self.ANSWERS[i])

        event_id = ReactionEvent.poll
        expires = datetime.datetime.now() + datetime.timedelta(days=1)
        payload = {
            'exclusive': exclusive,
            'title': title,
            'options': options   
        }
        await self.tb_react_events.insert(msg.id, msg.guild.id, msg.channel.id, event_id, json.dumps(payload), expires)

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
        return "**__%s__**\n**Time:** %s\n:white_check_mark: Yes (%d): %s\n:x: No (%d): %s\n:shrug: Maybe (%d): %s" % (
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
