import re
import discord
import lavalink
from discord.ext import commands
from src.utils import format_seconds

url_rx = re.compile(r'https?://(?:www\.)?.+')


class LavaMusic(commands.Cog, name="Voice"):
    def __init__(self, bot):
        self.bot = bot

    @commands.Cog.listener()
    async def on_ready(self):
        if not hasattr(self.bot, 'lavalink'):
            self.bot.lavalink = lavalink.Client(self.bot.user.id, shard_count=self.bot.shard_dict['shard_count'])
            self.bot.lavalink.add_node('lavalink', 2333, 'merryandpippinarecute', 'us' 'arch music')
            self.bot.add_listener(self.bot.lavalink.voice_update_handler, 'on_socket_response')
            lavalink.add_event_hook(self.track_hook)

    def cog_unload(self):
        self.bot.lavalink._event_hooks.clear()

    async def cog_before_invoke(self, ctx):
        guild_check = ctx.guild is not None

        if guild_check:
            await self.ensure_voice(ctx.author, ctx.guild, ctx.command.name in ('p', 'play'))

        return guild_check

    async def ensure_voice(self, user, guild, should_connect: bool):
        settings = self.bot.settings[guild]
        vol = settings.music_volume
        vol *= 1000

        if not settings.music_enabled:
            raise commands.CommandInvokeError('playing music is not enabled on this server')

        if settings.music_role and settings.music_role not in user.roles \
                and user.id not in settings.admin_ids:
                    raise commands.CommandInvokeError('must be part of the music role')

        if not user.voice or not user.voice.channel:
            raise commands.CommandInvokeError('need to be in a voice channel to play a song')

        player = self.bot.lavalink.player_manager.create(ctx.guild.id, endpoint=str(guild.region))
        player.set_volume(vol)
        if not player.isconnected:
            if not should_connect:
                raise commands.CommandInvokeError('architus needs to be connected to a voice channel')

            permissions = author.voice.channel.permissions_for(guild.me)
            if not permissions.connect or not permissions.speak:
                raise commands.CommandInvokeError('architus needs connect and speak permissions')

            await guild.change_voice_state(channel=user.voice.channel)
        else:
            if int(player.channel_id) != ctx.author.voice.channel.id:
                raise commands.CommandInvokeError('need to be in the same voice channel first')

    async def track_hook(self, event):
        if isinstance(event, lavalink.events.QueueEndEvent):
            guild_id = int(event.player.guild_id)
            guild = self.got.get_build(guild_id)
            await guild.change_voice_state(channel=None)

    @commands.command()
    async def skip(self, ctx):
        player = self.bot.lavalink.player_manager.get(ctx.guild.id)
        await player.skip()

    def queue_embed(self, p, q):
        songs = "\n".join(f"**{i+1:>2}.** *{song.title}*" for i, song in enumerate(q[:10]))
        if len(q) > 10:
            songs += "more not shown..."
        if p.is_playing:
            hour = p.current.duration > 3600
            title = p.current.title
            url = p.current.uri
            name = f"Now Playing ({format_seconds(p.position, hour)}/{format_seconds(p.current.position, hour)}):"
        else:
            title = "no songs queued"
            url = None
        em = discord.Embed(title=title, url=url, description=songs, color=0x6600ff)
        em.set_author(name=name)
        em.set_footer(text=f"🔀 Shuffle: {p.shuffle}")
        return em

    @commands.group(aliases=['q'])
    async def queue(self, ctx):
        if ctx.invoked_subcommand is None:
            p = self.bot.lavalink.player_manager.get(ctx.guild.id)
            await ctx.send(self.queue_embed(p, p.queue))

    @queue.command(aliases=['a'])
    async def add(self, ctx, *query):
        await self.play(ctx, query=query)

    @queue.command(aliases=['remove', 'r'])
    async def rm(self, ctx, index: int):
        '''queue rm <index>'''
        q = self.bot.lavalink.player_manager.get(ctx.guild.id).queue
        index = len(q) - index
        try:
            song = q[index]
            del q[index]
        except IndexError:
            await ctx.send("not sure what song to delete")
        else:
            await ctx.send(f"removed *{song.title}*")

    @queue.command()
    async def clear(self, ctx):
        manager = self.bot.lavalink.player_manager.get(ctx.guild.id)
        q = manager.queue
        manager.queue = []
        await ctx.send(f"cleared {len(q)} songs from the queue")

    @queue.command()
    async def shuffle(self, ctx):
        p = self.bot.lavalink.player_manager.get(ctx.guild.id)
        if p.shuffle:
            p.shuffle = False
            await ctx.send("Shuffle is **off**")
        else:
            p.shuffle = True
            await ctx.send("Shuffle is **on**")

    async def enqueue(self, query, user, guild):
        """
        Takes in the query, the user that asked, and which guild and returns a list of the
        songs that were added to the queue.
        """
        player = self.bot.lavalink.player_manager.get(guild.id)
        query = query.strip('<>')

        if not url_rx.match(query):
            query = f'tysearch:{query}'

        results = await player.node.get_tracks(query)

        if not results or not results['tracks']:
            # Return that nothing was found
            return []

        if results['loadType'] == 'PLAYLIST_LOADED':
            tracks = results['tracks']

            for track in tracks:
                player.add(requester=user.id, track=track)

            if not player.is_playing:
                await player.play()

            return ['playlist', results['playlistInfo']['name'], len(tracks)]
        else:
            song = results['tracks'][0]
            track = lavalink.models.AudioTrack(song, user.id, recommended=True)
            player.add(requester=user.id, track=track)

            if not player.is_playing:
                await player.play()

            return ['song', song]

    @commands.command(aliases=['p'])
    async def play(self, ctx, *, query: str):
        songs = self.enqueue(query, ctx.author, ctx.guild)

        if len(songs) == 0:
            return await ctx.send('no tracks found')

        embed = discord.Embed(color=discord.Color.blurple())
        if songs[0] == 'playlist':
            embed.title = 'Playlist Enqueued'
            embed.description = f'{songs[1]} - {songs[2]} tracks'
        else:
            track = songs[1]
            embed.title = 'Tracks enqueued'
            embed.description = f'[{track["info"]["title"]}]({track["info"]["uri"]})'
            if "youtube" in track.uri:
                yt_id = track.uri.split("=")[1]
                embed.set_thumbnail(url=f"http://img.youtube.com/vi/{yt_id}/1.jpg")

        await ctx.send(embed=embed)

    @commands.command(aliases=['dc'])
    async def stop_music(self, ctx):
        player = self.bot.lavalink.player_manager.get(ctx.guild.id)

        if not player.is_connected:
            return await ctx.send('not connected')

        if not ctx.author.voice or (player.is_connected and ctx.author.voice.channel.id != int(player.channel_id)):
            return await ctx.send('Not in my voice channel')

        player.queue.clear()
        await player.stop()
        await ctx.guild.change_voice_state(channel=None)
        await ctx.send('*⃣ | Disconnected.')


def setup(bot):
    bot.add_cog(LavaMusic(bot))
