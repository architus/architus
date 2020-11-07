from discord.ext import commands
import discord
import aiohttp
import json

from lib.config import domain_name, twitch_access_token, twitch_client_id, logger
from lib.aiomodels import TwitchStream
from datetime import datetime

class Twitch(commands.Cog, name="Twitch Notification"):

    def __init__(self, bot):
        self.bot = bot
        self.stream_list = {}        
        self.twitch_stream = TwitchStream(self.bot.asyncpg_wrapper)

    # @commands.command()
    # async def add_stream(self, ctx, stream_url):
    #     sub_hub = {
    #         "callback": "",
    #         "mode": "subscribe",
    #         "topic": ,
    #         "lease_seconds": 864000,
    #         "secret": 
    #     }

    @commands.command()
    async def addstream(self, ctx, username):
        async with aiohttp.ClientSession() as session:
            url = f"https://api.twitch.tv/helix/users?login={username}"
            headers = {
                'client-id': twitch_client_id,
                'Authorization': f'Bearer {twitch_access_token}'
            }
            async with session.get(url, headers=headers) as resp:
                user_fields = await resp.json()

            user_id = user_fields['data'][0]['id']
            user_display_name = user_fields['data'][0]['display_name']
            user_profile_image_url = user_fields['data'][0]['profile_image_url']
        
            await ctx.send(user_id)
            await ctx.send(user_profile_image_url)

            url = "https://api.twitch.tv/helix/webhooks/hub"
            hub = {
                "hub.callback": f"https://api.{domain_name}/twitch",
                "hub.mode": "subscribe",
                "hub.topic": f"https://api.twitch.tv/helix/streams?user_id={user_id}",
                "hub.lease_seconds": 864000,
                "hub.secret": "peepeepoopoo"
            }


            should_subscribe = len(await self.twitch_stream.select_by_stream_id(int(user_id))) == 0


            streams = await self.twitch_stream.select_by_guild(ctx.guild.id)
            for row in streams:
                if row["stream_user_id"] == int(user_id):
                    await ctx.send('go away stream is already in the table')
                    return
            await self.twitch_stream.insert({"stream_user_id": int(user_id), "guild_id": ctx.guild.id})

            if should_subscribe:
                async with session.post(url, data=hub, headers=headers) as resp:
                    statuss = resp.status
                print(statuss)
            
            if self.bot.settings[ctx.guild].twitch_channel_id is None:
                self.bot.settings[ctx.guild].twitch_channel_id = ctx.channel.id
                await ctx.send(f'Twitch updates bound to {ctx.channel.mention}.')
            
            await ctx.send('Successfully subscribed! :)')
            
    async def update(self, stream):
        rows = await self.twitch_stream.select_by_stream_id(int(stream['user_id']))
        guilds = {self.bot.get_guild(r['guild_id']) for r in rows}
        for guild in guilds:
            if guild is None:
                continue
            channel_id = self.bot.settings[guild].twitch_channel_id
            channel = guild.get_channel(channel_id)
            
            if channel is not None:
                game = await self.get_game(stream["game_id"])
                await channel.send(embed=self.embed_helper(stream, game))
                logger.debug(stream["type"])

    async def get_game(self, game_id):
        async with aiohttp.ClientSession() as session:
            url = f"https://api.twitch.tv/helix/games?id={game_id}"
            headers = {
                'client-id': twitch_client_id,
                'Authorization': f'Bearer {twitch_access_token}'
            }
            async with session.get(url, headers=headers) as resp:
                games = await resp.json()
        return games["data"][0]

    def embed_helper(self, stream, game):
        timestamp = datetime.fromisoformat(stream["started_at"][:-1])
        em = discord.Embed(title=stream["title"], url=f"https://twitch.tv/{stream['user_name']}", description=f"{stream['user_name']} is playing {game['name']}!", colour=0x6441A4, timestamp=timestamp)
        em.set_author(name=stream["user_name"], icon_url=stream["thumbnail_url"])
        # em.set_footer(text=timestamp)
        em.set_thumbnail(url=game["box_art_url"].format(width=130, height=180))

        return em


def setup(bot):
    bot.add_cog(Twitch(bot))
