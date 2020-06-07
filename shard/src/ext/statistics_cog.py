from collections import defaultdict
from datetime import timedelta
from concurrent.futures import ThreadPoolExecutor
import base64

from discord.ext import commands
from discord import Forbidden, HTTPException
import discord
import json
import aiohttp
import asyncio

import src.generate.wordcount as wordcount_gen
from src.generate import corona
from lib.config import DISCORD_EPOCH, logger
from lib.ipc import manager_pb2 as message_type


class MessageData:

    def __init__(self, message_id, author, channel_id, total_words, correct_words):
        self.message_id = message_id
        self.author = author
        self.channel_id = channel_id
        self.total_words = total_words
        self.correct_words = correct_words
        self.cache_up_to_date = {}

    def __hash__(self):
        return hash(self.message_id)

    @property
    def created_at(self):
        return DISCORD_EPOCH + timedelta(milliseconds=self.message_id >> 22)


class MessageStats(commands.Cog, name="Server Statistics"):

    def __init__(self, bot):
        self.bot = bot
        self.cache = defaultdict(list)
        with open('res/words/words.json') as f:
            self.dictionary = json.loads(f.read())

    def count_correct(self, string):
        '''returns the number of correctly spelled words in a string'''
        return len([w for w in string.split() if w in self.dictionary or w.upper() in ('A', 'I')])

    async def cache_channel(self, channel):
        async for message in channel.history(limit=None, oldest_first=True):
            self.cache[channel.guild.id].append(MessageData(
                message.id,
                message.author,
                channel.id,
                len(message.clean_content.split()),
                self.count_correct(message.clean_content)
            ))

    async def cache_guild(self, guild):
        '''cache interesting information about all the messages in a guild'''
        logger.debug(f"Downloading messages in {len(guild.channels)} channels for '{guild.name}'...")
        for channel in guild.text_channels:
            try:
                await self.cache_channel(channel)
            except Forbidden:
                logger.warning(f"Insuffcient permissions to download messages from '{guild.name}.{channel.name}'")
            except HTTPException as e:
                logger.error(f"Caught {e} when downloading '{guild.name}.{channel.name}'")
                logger.error("trying again in 10 seconds...")
                await asyncio.sleep(10)
                try:
                    await self.cache_channel(channel)
                except Exception:
                    logger.exception("failed to download channel a second time, giving up :(")

    @commands.Cog.listener()
    async def on_ready(self):
        logger.debug(f"Caching messages for {len(self.bot.guilds)} guilds...")
        for guild in self.bot.guilds:
            await self.cache_guild(guild)
        logger.debug(f"Message cache up-to-date for {len(self.bot.guilds)} guilds...")

    @commands.Cog.listener()
    async def on_message(self, msg):
        if not msg.channel.guild:
            return
        self.cache[msg.channel.guild.id].append(MessageData(
            msg.id,
            msg.author,
            msg.channel.id,
            len(msg.clean_content.split()),
            self.count_correct(msg.clean_content)
        ))

    @commands.Cog.listener()
    async def on_guild_join(self, guild):
        await self.cache_guild(guild)

    @commands.command(aliases=['exclude'])
    async def optout(self, ctx):
        """Prevents Architus from displaying statistics about you
        run again to reallow collection
        """
        settings = self.bot.settings[ctx.guild]
        excludes = settings.stats_exclude
        author = ctx.author
        if author.id in excludes:
            excludes.remove(author.id)
            await ctx.send(f"{author.display_name}'s message data is now available")
        else:
            excludes.append(author.id)
            await ctx.send(f"{author.display_name}'s message data is now hidden")
        settings.stats_exclude = excludes

    @commands.command()
    async def spellcheck(self, ctx, victim: discord.Member):
        '''Checks the spelling of a user'''
        if ctx.author.id in self.bot.settings[ctx.guild].stats_exclude:
            await ctx.send(f"Sorry, {victim.display_name} has requested that their stats not be recorded :confused:")
            return
        words = 1
        correct_words = 0
        async with ctx.channel.typing():
            for msgdata in self.cache[ctx.guild.id]:
                if msgdata.author.id == victim.id:
                    words += msgdata.total_words
                    correct_words += msgdata.correct_words
        ratio = correct_words / words * 100
        await ctx.send(f"{ratio:.1f}% of the {words:,} words sent by {victim.display_name} are spelled correctly.")

    async def count_messages(self, guild):
        '''Count the total messages a user has sent in the server'''
        word_counts = {}
        message_counts = {}
        for msgdata in self.cache[guild.id]:
            if msgdata.author.id in self.bot.settings[guild].stats_exclude:
                continue
            message_counts[msgdata.author] = message_counts.get(msgdata.author, 0) + 1
            word_counts[msgdata.author] = word_counts.get(msgdata.author, 0) + msgdata.total_words

        return message_counts, word_counts

    def bin_messages(self, guild, time_granularity: timedelta):
        time_bins = defaultdict(int)
        member_bins = defaultdict(int)
        channel_bins = defaultdict(int)
        for msgdata in self.cache[guild.id]:
            date = msgdata.created_at - ((msgdata.created_at - DISCORD_EPOCH) % time_granularity)
            time_bins[date.isoformat()] += 1
            if msgdata.author.id not in self.bot.settings[guild].stats_exclude:
                member_bins[msgdata.author.id] += 1
            channel_bins[msgdata.channel_id] += 1
        return member_bins, channel_bins, time_bins

    @commands.command()
    async def messagecount(self, ctx, victim: discord.Member = None):
        async with ctx.channel.typing():
            message_counts, word_counts = await self.count_messages(ctx.guild)

        with ThreadPoolExecutor() as pool:
            img = await self.bot.loop.run_in_executor(pool, wordcount_gen.generate, message_counts, word_counts, victim)
        data = await self.bot.manager_client.publish_file(
            message_type.File(file=img))

        em = discord.Embed(title="Top 5 Message Senders", description=ctx.guild.name)
        em.set_image(url=data.url)
        em.color = 0x7b8fb7
        if victim:
            if victim.id in self.bot.settings[ctx.guild].stats_exclude:
                em.set_footer(text=f"{victim.display_name} has hidden their stats")
            else:
                em.set_footer(text="{0} has sent {1:,} words across {2:,} messages".format(
                    victim.display_name, word_counts[victim], message_counts[victim]), icon_url=victim.avatar_url)

        await ctx.channel.send(embed=em)


class CoronaStats(commands.Cog, name="Coronavirus Data"):
    def __init__(self, bot):
        self.bot = bot

    @commands.command(aliases=['covid', 'covid-19', 'covid19', 'coronavirus'])
    async def corona(self, ctx, deaths_only: bool = False):
        async with aiohttp.ClientSession() as session:
            url = 'http://coronavirusapi.com/time_series.csv'
            async with session.get(url) as resp:
                text = await resp.text()
                parsed = (e.split(',') for e in text.split('\n')[1:-1])

                img = corona.generate(parsed, deaths_only)

                data = await self.bot.manager_client.publish_file(
                    message_type.File(file=img))

                em = discord.Embed(title="Coronavirus in the US", description="More Information",
                                   url="https://www.cdc.gov/coronavirus/2019-ncov/index.html")
                em.set_image(url=data.url)
                em.color = 0x7b8fb7
                em.set_footer(text="data collected from state websites by http://coronavirusapi.com")

                await ctx.channel.send(embed=em)


def setup(bot):
    bot.add_cog(MessageStats(bot))
    bot.add_cog(CoronaStats(bot))
