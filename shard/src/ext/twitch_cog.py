from discord.ext import commands
import discord
import aiohttp
import json
import asyncio

from lib.config import domain_name, twitch_access_token, twitch_client_id, logger
from lib.aiomodels import TwitchStream
from datetime import datetime

headers = {
    'client-id': twitch_client_id,
    'Authorization': f'Bearer {twitch_access_token}'
}

class Twitch(commands.Cog, name="Twitch Notification"):

    def __init__(self, bot):
        self.bot = bot
        self.stream_list = {}        
        self.twitch_stream = TwitchStream(self.bot.asyncpg_wrapper)

    @commands.Cog.listener()
    async def on_ready(self):
        async with aiohttp.ClientSession() as session:
            while True:
                subscribed_ids = await self.twitch_stream.select_distinct_stream_id()
                for row in subscribed_ids:
                    url = "https://api.twitch.tv/helix/webhooks/hub"
                    hub = {
                        "hub.callback": f"https://api.{domain_name}/twitch",
                        "hub.mode": "subscribe",
                        "hub.topic": f"https://api.twitch.tv/helix/streams?user_id={row['stream_user_id']}",
                        "hub.lease_seconds": 864000,
                        "hub.secret": "peepeepoopoo"
                    }
                    async with session.post(url, data=hub, headers=headers) as resp:
                        statuss = resp.status
                await asyncio.sleep(864000 / 2)
        
    async def get_info(self, username):
        async with aiohttp.ClientSession() as session:
            url = f"https://api.twitch.tv/helix/users?login={username}"
            
            async with session.get(url, headers=headers) as resp:
                user_fields = await resp.json()

            user_id = user_fields['data'][0]['id']
            user_display_name = user_fields['data'][0]['display_name']
            user_profile_image_url = user_fields['data'][0]['profile_image_url']

            return int(user_id), user_display_name, user_profile_image_url

    @commands.command()
    async def addstream(self, ctx, username):
        async with aiohttp.ClientSession() as session:
            user_id, user_display_name, _ = await self.get_info(username)

            url = "https://api.twitch.tv/helix/webhooks/hub"
            hub = {
                "hub.callback": f"https://api.{domain_name}/twitch",
                "hub.mode": "subscribe",
                "hub.topic": f"https://api.twitch.tv/helix/streams?user_id={user_id}",
                "hub.lease_seconds": 864000,
                "hub.secret": "peepeepoopoo"
            }

            streams = await self.twitch_stream.select_by_guild(ctx.guild.id)
            for row in streams:
                if row["stream_user_id"] == user_id:
                    await ctx.send('go away stream is already in the table')
                    return
            await self.twitch_stream.insert({"stream_user_id": user_id, "guild_id": ctx.guild.id})

            should_subscribe = len(await self.twitch_stream.select_distinct_by_stream_id(user_id)) == 0
            if should_subscribe:
                async with session.post(url, data=hub, headers=headers) as resp:
                    statuss = resp.status
                print(statuss)
            
            if self.bot.settings[ctx.guild].twitch_channel_id is None:
                self.bot.settings[ctx.guild].twitch_channel_id = ctx.channel.id
                await ctx.send(f'Twitch updates bound to {ctx.channel.mention}.')
            
            await ctx.send(f'Successfully subscribed to {user_display_name}! :)')

    @commands.command()
    async def removestream(self, ctx, username):
        user_id, user_display_name, _ = await self.get_info(username)
        
        await self.twitch_stream.delete_by_stream_id(user_id, ctx.guild.id)            
        await ctx.send(f'Successfully unsubscribed from {user_display_name}! :)')

    @commands.command()
    async def whichstreams(self, ctx):
        em = discord.Embed(title="Subscribed Streams", colour=0x6441A4)        
        em.set_thumbnail(url="https://cdn.discordapp.com/attachments/715687026195824771/775244694066167818/unknown.png")

        streams = await self.twitch_stream.select_by_guild(ctx.guild.id)
        user_info = await self.get_users([str(row["stream_user_id"]) for row in streams])
        stream_info = await self.get_streams([str(row["stream_user_id"]) for row in streams])
        user_and_stream = {stream["user_id"]:stream for stream in stream_info}
        for user in user_info:
            try:
                stream = user_and_stream[user["id"]]                
                live = ":green_circle: Online"
            except KeyError:
                live = ":red_circle: Offline"
            em.add_field(name=user["display_name"], value=live, inline=True)

        await ctx.send(embed=em)
        
    async def update(self, stream):
        rows = await self.twitch_stream.select__distinct_by_stream_id(int(stream['user_id']))
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

    async def get_users(self, stream_user_ids):
        peepee = "&id=".join(stream_user_ids)
        async with aiohttp.ClientSession() as session:
            url = f"https://api.twitch.tv/helix/users?id={peepee}"
            async with session.get(url, headers=headers) as resp:
                info = await resp.json()
        return info["data"]

    async def get_streams(self, stream_user_ids):
        peepee = "&user_id=".join(stream_user_ids)
        async with aiohttp.ClientSession() as session:
            url = f"https://api.twitch.tv/helix/streams?user_id={peepee}"
            async with session.get(url, headers=headers) as resp:
                info = await resp.json()
        return info["data"]

    async def get_game(self, game_id):
        async with aiohttp.ClientSession() as session:
            url = f"https://api.twitch.tv/helix/games?id={game_id}"
            async with session.get(url, headers=headers) as resp:
                games = await resp.json()
        return games["data"][0]

    def embed_helper(self, stream, game):
        timestamp = datetime.fromisoformat(stream["started_at"][:-1])
        em = discord.Embed(title=stream["title"], url=f"https://twitch.tv/{stream['user_name']}", description=f"{stream['user_name']} is playing {game['name']}!", colour=0x6441A4, timestamp=timestamp)
        em.set_author(name=stream["user_name"], icon_url=stream["thumbnail_url"])
        em.set_thumbnail(url=game["box_art_url"].format(width=130, height=180))

        return em


def setup(bot):
    bot.add_cog(Twitch(bot))
