from discord.ext import commands
import random, string, os
import src.generate.wordcount as wordcount_gen
import discord
import json

IMAGE_CHANNEL_ID = 577523623355613235
LINHS_ID = 81231616772411392

class MessageStats(commands.Cog):

    def __init__(self, bot):
        self.bot = bot
        self._cache = None
        with open('res/words/words.json') as f:
            self.dictionary = json.loads(f.read())

    @property
    def cache(self):
        self._cache = self._cache or {guild: {} for guild in self.bot.guilds}
        return self._cache

    @commands.Cog.listener()
    async def on_message(self, msg):
        try:
            self.cache[msg.guild]['messages'][msg.channel].append(msg)
        except (KeyError, AttributeError):
            pass

    @commands.Cog.listener()
    async def on_guild_join(self, guild):
        self.cache.setdefault(ctxchannel.guild, {})


    @commands.command()
    async def spellcheck(self, ctx, victim: discord.Member):
        '''Checks the spelling of a user'''
        ctxchannel = ctx.channel
        cache = self.cache
        cache[ctxchannel.guild].setdefault('messages', {})
        blacklist = []
        blacklist.append(discord.utils.get(ctx.guild.text_channels, name='bot-commands'))
        blacklist.append(discord.utils.get(ctx.guild.text_channels, name='private-bot-commands'))
        correct_words = 0
        words = 1
        async with ctxchannel.typing():
            for channel in ctx.guild.text_channels:
                try:
                    if not channel in blacklist:
                        if not channel in cache[ctxchannel.guild]['messages'].keys() or not cache[ctxchannel.guild]['messages'][channel]:
                            print("reloading cache for " + channel.name)
                            iterator = [log async for log in channel.history(limit=7500)]
                            logs = list(iterator)
                            cache[ctxchannel.guild]['messages'][channel] = logs
                        msgs = cache[ctxchannel.guild]['messages'][channel]
                        for msg in msgs:
                            if msg.author == victim:
                                for word in msg.clean_content.split():
                                    if word[0] == '!':
                                        continue
                                    words += 1
                                    if word in self.dictionary and len(word) > 1 or word in ['a','i', 'A', 'I']:
                                        correct_words += 1
                except Exception as e:
                    print(e)
        linh_modifier = 10 if victim.id == LINHS_ID else 0
        await ctx.channel.send("{0:.1f}% out of the {1:,} scanned words sent by {2} are spelled correctly".format(
            ((correct_words/words)*100) - linh_modifier, words, victim.display_name))

    @commands.command()
    async def messagecount(self, ctx, *args):
        '''Count the total messages a user has sent in the server'''
        ctxchannel = ctx.channel
        cache = self.cache
        cache[ctxchannel.guild].setdefault('messages', {})
        blacklist = []
        word_counts = {}
        message_counts = {}
        victim = ctx.message.mentions[0] if ctx.message.mentions else None
        async with ctxchannel.typing():
            for channel in ctx.guild.text_channels:
                try:
                    if not channel in blacklist:
                        if not channel in cache[ctxchannel.guild]['messages'].keys() or not cache[ctxchannel.guild]['messages'][channel]:
                            print("reloading cache for " + channel.name)
                            iterator = [log async for log in channel.history(limit=1000000)]
                            logs = list(iterator)
                            cache[ctxchannel.guild]['messages'][channel] = logs
                        msgs = cache[ctxchannel.guild]['messages'][channel]
                        for msg in msgs:
                            message_counts[msg.author] = (message_counts[msg.author] if msg.author in message_counts else 0) + 1
                            word_counts[msg.author] = (word_counts[msg.author] if msg.author in word_counts else 0) + len(msg.clean_content.split())
                except Exception as e:
                    print(e)

        key = ''.join(random.choice(string.ascii_letters) for n in range(10))
        wordcount_gen.generate(key, message_counts, word_counts, victim)
        channel = discord.utils.get(self.bot.get_all_channels(), id=IMAGE_CHANNEL_ID)

        with open(f'res/word{key}.png', 'rb') as f:
            msg = await channel.send(file=discord.File(f))

        em = discord.Embed(title="Top 5 Message Senders", description=ctx.guild.name)
        em.set_image(url=msg.attachments[0].url)
        em.color = 0x7b8fb7
        if victim:
            em.set_footer(text="{0} has sent {1:,} words across {2:,} messages".format(victim.display_name, word_counts[victim], message_counts[victim]), icon_url=victim.avatar_url)

        await ctx.channel.send(embed=em)

        os.remove(f"res/word{key}.png")

def setup(bot):
    bot.add_cog(MessageStats(bot))
