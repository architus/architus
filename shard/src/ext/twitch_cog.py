from discord.ext import commands
import discord
import aiohttp
import asyncio
from typing import List

from lib.config import domain_name, twitch_client_secret, twitch_client_id, logger, twitch_hub_secret
from lib.aiomodels import TwitchStream, Tokens
from datetime import datetime, timedelta


class Twitch(commands.Cog, name="Twitch Notification"):

    def __init__(self, bot):
        self.bot = bot
        self.twitch_stream = TwitchStream(self.bot.asyncpg_wrapper)
        self.tokens = Tokens(self.bot.asyncpg_wrapper)

    async def get_headers(self):
        token_stuff = (await self.tokens.select_by_id({"client_id": twitch_client_id}))["client_token"]
        headers = {
            'client-id': twitch_client_id,
            'Authorization': f'Bearer {token_stuff}',
            'Content-Type': 'application/json',
        }
        return headers

    async def sub_by_id(self, user_id, username=''):
        url = "https://api.twitch.tv/helix/eventsub/subscriptions"
        data = {
            "type": "stream.online",
            "version": "1",
            "condition": {
                "broadcaster_user_id": str(user_id)
            },
            "transport": {
                "method": "webhook",
                "callback": f"https://api.{domain_name}/twitch",
                "secret": twitch_hub_secret
            }
        }

        async with aiohttp.ClientSession() as session:
            async with session.post(url, json=data, headers=await self.get_headers()) as resp:
                logger.info(f"attempted to subscribe to {username}({user_id}), received {resp.status}")
                if resp.status != 202:
                    logger.debug(await resp.text())

    async def log_subs(self):
        url = 'https://api.twitch.tv/helix/eventsub/subscriptions'
        async with aiohttp.ClientSession() as session:
            async with session.get(url, headers=await self.get_headers()) as resp:
                logger.debug(f'twitch subs: {await resp.text()}')


    @commands.Cog.listener()
    async def on_ready(self):
        while self.bot.shard_id == 0:
            await self.refresh_token()
            subscribed_ids = await self.twitch_stream.select_distinct_stream_id()
            await self.log_subs()
            for row in subscribed_ids:
                try:
                    await self.sub_by_id(row['stream_user_id'])
                except Exception:
                    logger.exception(f"Error resubscribing to {row['stream_user_id']}")
            await asyncio.sleep(864000 / 2)

    async def get_info(self, username):
        async with aiohttp.ClientSession() as session:
            url = f"https://api.twitch.tv/helix/users?login={username}"

            async with session.get(url, headers=await self.get_headers()) as resp:
                user_fields = await resp.json()

            if 'data' not in user_fields or user_fields['data'] == []:
                return None

            user_id = user_fields['data'][0]['id']
            user_display_name = user_fields['data'][0]['display_name']
            user_profile_image_url = user_fields['data'][0]['profile_image_url']

            return int(user_id), user_display_name, user_profile_image_url

    async def get_validated_info(self, ctx, username: str = ''):
        if username == '':
            await ctx.send('Please pass in a valid username.')
            raise commands.CommandInvokeError("no username given")

        result = await self.get_info(username)
        if result is None:
            await ctx.send(f"Couldn't find a stream called {username}", allowed_mentions=discord.AllowedMentions.none())
            raise commands.CommandInvokeError("no stream associated with given username")

        return result

    @commands.command()
    async def subscribe(self, ctx, username: str = ''):
        user_id, user_display_name, _ = await self.get_validated_info(ctx, username)

        rows = await self.twitch_stream.select_by_guild(ctx.guild.id)
        if any(r['stream_user_id'] == user_id for r in rows):
            await ctx.send(f'Already subscribed to {username}')
            return

        await self.twitch_stream.insert({"stream_user_id": user_id, "guild_id": ctx.guild.id})

        # aka whether architus as a whole is already subscribed to the requested channel
        if len(await self.twitch_stream.select_distinct_by_stream_id(user_id)) == 0:
            await self.sub_by_id(user_id, username=user_display_name)

        if self.bot.settings[ctx.guild].twitch_channel_id is None:
            self.bot.settings[ctx.guild].twitch_channel_id = ctx.channel.id
            await ctx.send(f'Twitch updates bound to {ctx.channel.mention}.')

        await ctx.send(f'Successfully subscribed to {user_display_name}! :)')

    @commands.command()
    async def unsubscribe(self, ctx, username: str = ''):
        user_id, user_display_name, _ = await self.get_validated_info(ctx, username)

        await self.twitch_stream.delete_by_stream_id(user_id, ctx.guild.id)
        await ctx.send(f'Successfully unsubscribed from {user_display_name}! :)')

    @commands.command()
    async def streams(self, ctx):
        em = discord.Embed(title="Subscribed Streams", colour=0x6441A4)
        em.set_thumbnail(url="https://cdn.discordapp.com/attachments/715687026195824771/775244694066167818/unknown.png")

        streams = await self.twitch_stream.select_by_guild(ctx.guild.id)
        if len(streams) == 0:
            await ctx.send("Not subscribed to any streams.")
            return
        user_info = await self.get_users([str(row["stream_user_id"]) for row in streams])
        stream_info = await self.get_streams([str(row["stream_user_id"]) for row in streams])
        user_and_stream = {stream["user_id"]: stream for stream in stream_info}
        for user in user_info:
            try:
                user_and_stream[user["id"]]
                live = ":green_circle: Online"
            except KeyError:
                live = ":red_circle: Offline"
            em.add_field(name=user["display_name"], value=live, inline=True)

        await ctx.send(embed=em)

    async def update(self, data):
        rows = await self.twitch_stream.select_distinct_by_stream_id(int(data['broadcaster_user_id']))
        guilds = {self.bot.get_guild(r['guild_id']) for r in rows}
        users = await self.get_users((str(data['broadcaster_user_id']),))
        
        stream = (await self.get_streams((data['broadcaster_user_id'],)))[0]

        for guild in guilds:
            if guild is None:
                continue
            channel_id = self.bot.settings[guild].twitch_channel_id
            channel = guild.get_channel(channel_id)

            if channel is not None:
                await channel.send(embed=self.embed_helper(stream, users[0]))
                logger.debug(stream["type"])

    async def get_users(self, stream_user_ids):
        peepee = "&id=".join(stream_user_ids)
        async with aiohttp.ClientSession() as session:
            url = f"https://api.twitch.tv/helix/users?id={peepee}"
            async with session.get(url, headers=await self.get_headers()) as resp:
                info = await resp.json()
        return info["data"]

    async def get_streams(self, stream_user_ids: List[int]):
        peepee = "&user_id=".join(stream_user_ids)
        async with aiohttp.ClientSession() as session:
            url = f"https://api.twitch.tv/helix/streams?user_id={peepee}"
            logger.debug(f'url: {url}')
            async with session.get(url, headers=await self.get_headers()) as resp:
                info = await resp.json()
                logger.debug(f'info: {info}')
        return info["data"]

    async def get_game(self, game_id: str):
        async with aiohttp.ClientSession() as session:
            url = f"https://api.twitch.tv/helix/games?id={game_id}"
            async with session.get(url, headers=await self.get_headers()) as resp:
                games = await resp.json()
        return games["data"][0]

    def embed_helper(self, stream, user):
        timestamp = datetime.fromisoformat(stream["started_at"][:-1])
        em = discord.Embed(
            title=stream["title"],
            url=f"https://twitch.tv/{stream['user_name']}",
            description=f"{stream['user_name']} is playing {stream['game_name']}!",
            colour=0x6441A4, timestamp=timestamp)

        em.set_author(name=stream["user_name"], icon_url=user["profile_image_url"])
        em.set_thumbnail(url=stream["thumbnail_url"].format(width=130, height=180))

        return em

    async def refresh_token(self):
        row = await self.tokens.select_by_id({"client_id": twitch_client_id})
        logger.info("Checking to refresh Twitch token...")
        if row is None or row["expires_at"] < datetime.now() + timedelta(days=10):
            async with aiohttp.ClientSession() as session:
                url = f"https://id.twitch.tv/oauth2/token?client_id={twitch_client_id}" \
                      f"&client_secret={twitch_client_secret}&grant_type=client_credentials"
                async with session.post(url) as resp:
                    info = await resp.json()

            await self.tokens.update_tokens(
                twitch_client_id, info["access_token"],
                datetime.now() + timedelta(seconds=info["expires_in"]))
            logger.info("Refreshed Twitch token")
        else:
            logger.info("Didn't need to be refreshed")


def setup(bot):
    bot.add_cog(Twitch(bot))
