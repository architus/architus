import datetime
import discord
import pytz
import dateutil.parser
import re
import json
from enum import IntEnum
from unidecode import unidecode
from contextlib import suppress
from discord.ext.commands import Cog
from discord.ext import commands
from src.utils import doc_url

from lib.config import logger
from lib.aiomodels import TbReactEvents


class ScheduleEvent(object):
    def __init__(self, msg, title, time_str):
        self.msg = msg
        self.title_str = title
        self.parsed_time = time_str
        self.yes = set()
        self.no = set()
        self.maybe = set()


class PollEvent(object):
    def __init__(self, msg, title, options, votes, exclusive):
        self.msg = msg
        self.title = title
        self.options = options
        self.votes = votes
        self.exclusive = exclusive


class ReactionEventType(IntEnum):
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
        self.schedule_messages = {}
        self.poll_messages = {}
        self.tb_react_events = TbReactEvents(self.bot.asyncpg_wrapper)

    @commands.Cog.listener()
    async def on_raw_reaction_add(self, payload):
        await self.on_reaction_update(payload, True)

    @commands.Cog.listener()
    async def on_raw_reaction_remove(self, payload):
        await self.on_reaction_update(payload, False)

    async def on_reaction_update(self, payload, add):
        '''
        Lumps together on_reaction_add and on_reaction_remove into one thing.
        add: True means its reaction_add, False, means reaction_remove
        '''
        channel = self.bot.get_channel(payload.channel_id)
        if channel is None:
            channel = await self.bot.fetch_channel(payload.channel_id)

        message = discord.utils.find(lambda m: m.id == payload.message_id, self.bot.cached_messages)
        if message is None:
            message = await channel.fetch_message(payload.message_id)

        user = payload.member
        emoji = payload.emoji
        if user is not None:
            if user.bot:
                return

        react_event = await self.tb_react_events.select_by_id(message.id, message.guild.id)
        if react_event is None:
            return

        react_event_payload = json.loads(react_event['payload'])
        react_event_type = react_event['event_type']
        if react_event_type == ReactionEventType.schedule:
            return
        elif react_event_type == ReactionEventType.poll:
            await self.poll_react_update(emoji, user, message, react_event_payload, add)
            return

    async def poll_react_update(self, emoji, user, message, react_event_payload, add):
        options = react_event_payload['options']
        exclusive = react_event_payload['exclusive']

        if add and exclusive:
            for r in message.reactions:
                if not str(r.emoji) == str(emoji) and user in await r.users().flatten():
                    await message.remove_reaction(r, user)

        votes = [[] for x in range(len(options))]
        for i in range(len(options)):
            reaction = next(r for r in message.reactions if r.emoji == self.ANSWERS[i])
            votes[i] = [u for u in await reaction.users().flatten() if not u.bot]

        text = self.render_poll_text(react_event_payload['title'], options, votes)
        await message.edit(content=text)

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        # polls_v1 logic
        if not user.bot and react.message.id in self.schedule_messages:
            event = self.schedule_messages[react.message.id]
            with suppress(KeyError):
                event.yes.remove(user)
            with suppress(KeyError):
                event.no.remove(user)
            with suppress(KeyError):
                event.maybe.remove(user)
            for r in react.message.reactions:
                if r != react:
                    await r.remove(user)

            if self.YES_EMOJI in str(react.emoji):
                event.yes.add(user)
            elif self.NO_EMOJI in str(react.emoji):
                event.no.add(user)
            elif self.MAYBE_EMOJI in str(react.emoji):
                event.maybe.add(user)
            await react.message.edit(
                content=self.render_schedule_text(event.title_str, event.parsed_time, event.yes, event.no, event.maybe))

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
    @doc_url("https://docs.archit.us/commands/events/#schedule")
    async def schedule(self, ctx, *argst):
        '''schedule <title> <event>
        Start an event poll.
        Timezone is based on your server's voice zone.
        '''
        args = list(argst)
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
        self.schedule_messages[msg.id] = ScheduleEvent(msg, title_str, parsed_time)

    @commands.command()
    @doc_url("https://docs.archit.us/commands/events/#poll")
    async def poll(self, ctx, *options):
        '''poll title, option1, option2, ..., option10
        Starts a poll with up to 10 options
        '''
        options = [o.strip() for o in ' '.join(options).split(',')]
        if len(options) < 2:
            await ctx.send("Please specify at least 1 option")
            return
        await self.register_poll_v2(ctx, options[0], options[1:], False)

    @commands.command()
    @doc_url("https://docs.archit.us/commands/events/#xpoll")
    async def xpoll(self, ctx, *options):
        '''xpoll title, option1, option2, ..., option10
        Starts an exclusive poll with up to 10 options
        '''
        options = [o.strip() for o in ' '.join(options).split(',')]
        if len(options) < 2:
            await ctx.send("Please specify at least 1 option")
            return
        await self.register_poll_v2(ctx, options[0], options[1:], True)

    async def register_poll_v2(self, ctx, title, options, exclusive: bool):
        votes = [[] for _ in range(10)]
        text = self.render_poll_text(title, options, votes)
        msg = await ctx.send(text)
        for i in range(len(options)):
            await msg.add_reaction(self.ANSWERS[i])

        event_id = int(ReactionEventType.poll)
        expires = datetime.datetime.now() + datetime.timedelta(days=24)
        payload = {
            'exclusive': exclusive,
            'title': title,
            'options': options
        }
        await self.tb_react_events.insert(msg.id, msg.guild.id, msg.channel.id, event_id, json.dumps(payload), expires)

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

        self.poll_messages[msg.id] = PollEvent(msg, title, options, votes, exclusive)

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
