from discord.ext.commands import Cog, Bot
from discord.ext import commands
from multiprocessing import Process, Queue
from src.config import scraper_token

import asyncio
import discord


class ScrimScraper(Bot):
    async def on_ready(self):
        print(f"Scraper logged in as {self.user.name}")

    async def on_message(self, msg):
        if msg.author == self.user:
            return
        if self.is_scrim_listing(msg) and self.is_na_listing(msg):
            info = {
                'content': msg.content,
                'author': {
                    'name': msg.author.name,
                    'discriminator': msg.author.discriminator,
                    'url': str(msg.author.avatar_url)
                },
                'timestamp': msg.created_at,
                'guild': {
                    'id': msg.guild.id,
                    'name': msg.guild.name,
                    'url': str(msg.guild.icon_url)
                }
            }
            self.q.put(info)

    def is_scrim_listing(self, msg):
        return "lfs" in msg.content.lower()

    def is_na_listing(self, msg):
        return 'na' in msg.channel.name.lower()



class ScrimFinderCog(Cog, name="Scrim Finder"):
    '''Scans some servers for 4k+ scrims'''

    def __init__(self, bot):
        self.bot = bot
        ss = ScrimScraper(('!',))
        self.q = Queue()
        ss.q = self.q
        p = Process(target=ss.run, args=(scraper_token,), kwargs={'bot': False})
        p.daemon = True
        p.start()

    @property
    def guild_settings(self):
        return self.bot.get_cog('GuildSettings')

    @commands.Cog.listener()
    async def on_ready(self):
        while True:
            while not self.q.empty():
                item = self.q.get()
                if self.is_in_range(item):
                    guild = self.bot.get_guild(item['guild']['id'])
                    settings = self.guild_settings.get_guild(guild)

                    channel = self.bot.get_channel(settings.scrim_channel_id)
                    await channel.send(embed=self.get_embed(item))
            await asyncio.sleep(5)

    def is_in_range(self, item):
        return '4' in item['content']

    def get_embed(self, item):
        em = discord.Embed(title="LFS Scrim", description=item['content'], colour=0x800080)
        em.set_author(name=item['guild']['name'], icon_url=item['guild']['url'])
        name = f"{item['author']['name']}#{item['author']['discriminator']}"
        em.set_footer(text=name, icon_url=item['author']['url'])
        return em



def teardown(bot):
    print("Terminating scraper bot")
    p.terminate()

def setup(bot):
    bot.add_cog(ScrimFinderCog(bot))
