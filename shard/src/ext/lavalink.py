import re
from typing import Optional
from lib.config import logger
from src.utils import format_seconds, doc_url

import discord
import lavalink
from discord.ext import commands
import spotipy
import spotipy.oauth2 as oauth2

url_rx = re.compile(r'https?://(?:www\.)?.+')
spotify_uri = re.compile(r'(?:https://open.spotify.com/(\w+)/([^?]+).*)|(?:spotify:(\w+):(\w+))')
hours = re.compile(r'(\d+):(\d+):(\d+)')
minutes = re.compile(r'(\d+):(\d+)')

MAX_PLAYLIST_LEN = 200


def gen_spotify_token():
    credentials = oauth2.SpotifyClientCredentials(
        client_id='1b4fadd2be3b48bda672c6e67caf7957',
        client_secret='051114b9930f491d86379338185148b5')
    return credentials.get_access_token()


def get_millis(time: str) -> Optional[int]:
    if (m := hours.match(time)) is not None:
        seconds = int(m.group(3))
        seconds += int(m.group(2)) * 60
        seconds += int(m.group(1)) * 3600
    elif (m := minutes.match(time)) is not None:
        seconds = int(m.group(2))
        seconds += int(m.group(1)) * 60
    else:
        try:
            seconds = int(time)
        except ValueError:
            return None
    return seconds * 1000


class LavaMusic(commands.Cog, name="Voice"):
    def __init__(self, bot):
        self.bot = bot
        try:
            self.spotify_client = spotipy.Spotify(auth=gen_spotify_token())
        except Exception:
            self.spotify_client = None
            logger.exception('')

    @commands.Cog.listener()
    async def on_ready(self):
        if not hasattr(self.bot, 'lavalink'):
            self.bot.lavalink = lavalink.Client(self.bot.user.id)
            self.bot.lavalink.add_node('lavalink', 2333, 'merryandpippinarecute', 'us' 'arch music')
            self.bot.add_listener(self.bot.lavalink.voice_update_handler, 'on_socket_response')
            lavalink.add_event_hook(self.track_hook)

    def cog_unload(self):
        self.bot.lavalink._event_hooks.clear()

    async def cog_before_invoke(self, ctx):
        guild_check = ctx.guild is not None

        if guild_check:
            try:
                await self.ensure_voice(ctx.author, ctx.guild, ctx.command.name in ('p', 'play'))
            except commands.CommandInvokeError as e:
                if e.original:
                    await ctx.send(e.original)
                raise e

        return guild_check

    async def ensure_voice(self, user, guild, should_connect: bool):
        settings = self.bot.settings[guild]
        vol = settings.music_volume
        vol *= 300
        vol = min(1000, max(0, int(vol)))

        if not settings.music_enabled:
            # no error message cause the only reason to disable music is for another bot
            raise commands.CommandInvokeError('')

        if settings.music_role and settings.music_role not in user.roles \
                and user.id not in settings.admin_ids:
            raise commands.CommandInvokeError('must be part of the music role')

        if not user.voice or not user.voice.channel:
            raise commands.CommandInvokeError('need to be in a voice channel to play a song')

        player = self.bot.lavalink.player_manager.create(guild.id, endpoint=str(guild.region))
        await player.set_volume(vol)
        if not player.is_connected:
            if not should_connect:
                raise commands.CommandInvokeError('architus needs to be connected to a voice channel')

            permissions = user.voice.channel.permissions_for(guild.me)
            if not permissions.connect or not permissions.speak:
                raise commands.CommandInvokeError('architus needs connect and speak permissions')

            await guild.change_voice_state(channel=user.voice.channel)
        else:
            if int(player.channel_id) != user.voice.channel.id:
                raise commands.CommandInvokeError('need to be in the same voice channel first')

    async def track_hook(self, event):
        if isinstance(event, lavalink.events.QueueEndEvent):
            guild_id = int(event.player.guild_id)
            guild = self.bot.get_guild(guild_id)
            await guild.change_voice_state(channel=None)

    @commands.command()
    @doc_url("https://docs.archit.us/commands/music/#skip")
    async def skip(self, ctx):
        '''skip
        Skips the current playing song and starts the next song in the queue if any
        '''
        player = self.bot.lavalink.player_manager.get(ctx.guild.id)
        await player.skip()
        if player.is_playing:
            track = player.current
            embed = discord.Embed(color=discord.Color.blurple())
            embed.title = 'Current Song'
            embed.description = f'[{track.title}]({track.uri})'
            if "youtube" in track.uri:
                yt_id = track.uri.split("=")[1]
                embed.set_thumbnail(url=f"https://img.youtube.com/vi/{yt_id}/1.jpg")
            await ctx.send(embed=embed)
        else:
            await player.stop()
            await ctx.guild.change_voice_state(channel=None)
            await ctx.send('no more songs left, goodbye')

    def queue_embed(self, p, q):
        songs = "\n".join(f"**{i+1:>2}.** *{song.title}*" for i, song in enumerate(q[:10]))
        if len(q) > 10:
            songs += "\nmore not shown..."
        if p.is_playing:
            duration = p.current.duration // 1000
            position = p.position // 1000
            hour = duration > 3600
            title = p.current.title
            url = p.current.uri
            name = f"Now Playing ({format_seconds(position, hour)}/{format_seconds(duration, hour)}):"
        else:
            title = "no songs queued"
            url = None
        em = discord.Embed(title=title, url=url, description=songs, color=0x6600ff)
        em.set_author(name=name)
        em.set_footer(text=f"🔀 Shuffle: {p.shuffle}")
        return em

    @commands.group(aliases=['q'])
    @doc_url("https://docs.archit.us/commands/#queue")
    async def queue(self, ctx):
        '''queue
        Show the current queue of songs that is playing
        '''
        if ctx.invoked_subcommand is None:
            p = self.bot.lavalink.player_manager.get(ctx.guild.id)
            await ctx.send(embed=self.queue_embed(p, p.queue))

    @queue.command(aliases=['a'])
    @doc_url("https://docs.archit.us/commands/#queue")
    async def add(self, ctx, *query):
        '''queue add <search item|url>
        Add songs to the queue.
        See help for the play command for more info.
        '''
        await self.play(ctx, query=" ".join(query))

    @queue.command(aliases=['remove', 'r'])
    @doc_url("https://docs.archit.us/commands/#queue")
    async def rm(self, ctx, index: int):
        '''queue rm <index>
        Remove some song from the queue.'''
        q = self.bot.lavalink.player_manager.get(ctx.guild.id).queue
        # index = len(q) - index - 1
        index = index - 1
        try:
            song = q[index]
            del q[index]
        except IndexError:
            await ctx.send("not sure what song to delete")
        else:
            await ctx.send(f"removed *{song.title}*")

    @queue.command()
    @doc_url("https://docs.archit.us/commands/music/#clear")
    async def clear(self, ctx):
        '''queue clear
        Clears the queue of all songs. Will keep playing the current song.
        '''
        manager = self.bot.lavalink.player_manager.get(ctx.guild.id)
        q = manager.queue
        manager.queue = []
        await ctx.send(f"cleared {len(q)} songs from the queue")

    @queue.command()
    async def shuffle(self, ctx):
        '''shuffle
        Toggles the current shuffle state. The current shuffle state can be seen by running the `queue` command.
        '''
        p = self.bot.lavalink.player_manager.get(ctx.guild.id)
        if p.shuffle:
            p.shuffle = False
            await ctx.send("Shuffle is **off**")
        else:
            p.shuffle = True
            await ctx.send("Shuffle is **on**")

    @queue.command()
    async def limit(self, ctx):
        '''limit
        See the current maximum number of songs that can be in the queue
        '''
        await ctx.send(f"{MAX_PLAYLIST_LEN} songs can be in the queue")

    @commands.command()
    @doc_url("https://docs.archit.us/commands/#seek")
    async def seek(self, ctx, location):
        '''seek <time>
        Go to a specific timestamp in a song.
        Time should be in one of the following formats: hh:mm:ss, mm:ss, ss
        '''
        player = self.bot.lavalink.player_manager.get(ctx.guild.id)
        if not player.is_playing:
            await ctx.send("song needs to be playing for this command")
            return
        time = get_millis(location)
        if time is None:
            await ctx.send("please send a valid time")
            return
        if time > player.current.duration:
            await ctx.send("time can't be longer than the song duration")
            return
        await player.seek(time)

    async def enqueue(self, query, user, guild):
        """
        Takes in the query, the user that asked, and which guild and returns a list of the
        songs that were added to the queue.
        """
        query = query.strip('<>')
        player = self.bot.lavalink.player_manager.get(guild.id)
        curr_length = len(player.queue)

        if 'spotify' in query:
            if self.spotify_client is None:
                return ['error', 'error connecting to spotify']
            if (match := spotify_uri.match(query)) is None:
                return []
            (track_type, uri) = match.group(1, 2) if match.group(1) is not None else match.group(3, 4)
            try:
                if track_type == "album":
                    album = self.spotify_client.album(uri)
                    artist = album['artists'][0]['name']
                    num_songs = len(album['tracks']['items'])
                    to_add = MAX_PLAYLIST_LEN - curr_length
                    for t in album['tracks']['items'][:to_add]:
                        await self.enqueue(f"{t['name']} by {artist}", user, guild)
                    return ['album', album['name'], min(num_songs, to_add), to_add < num_songs]
                elif track_type == "playlist":
                    playlist = self.spotify_client.playlist(uri)
                    num_songs = len(playlist['tracks']['items'])
                    to_add = MAX_PLAYLIST_LEN - curr_length
                    for t in playlist['tracks']['items'][:to_add]:
                        await self.enqueue(f"{t['track']['name']} by {t['track']['artists'][0]['name']}", user, guild)
                    return ['playlist', playlist['name'], min(num_songs, to_add), to_add < num_songs]
                elif track_type == "track":
                    track = self.spotify_client.track(uri)
                    name = track['name']
                    artist = track['artists'][0]['name']
                    return await self.enqueue(f"{name} by {artist}", user, guild)
                else:
                    return []
            except spotipy.SpotifyException:
                logger.exception('')
                return ['error', 'spotify api is down']

        if not url_rx.match(query):
            query = f'ytsearch:{query}'

        results = await player.node.get_tracks(query)

        if not results or not results['tracks']:
            # Return that nothing was found
            return []

        if results['loadType'] == 'PLAYLIST_LOADED':
            tracks = results['tracks']

            to_add = len(tracks)
            to_add = MAX_PLAYLIST_LEN - len(player.queue)
            for track in tracks[:to_add]:
                player.add(requester=user.id, track=track)

            if not player.is_playing:
                await player.play()

            return ['playlist', results['playlistInfo']['name'], min(len(tracks), to_add), len(tracks) > to_add]
        else:
            song = results['tracks'][0]
            track = lavalink.models.AudioTrack(song, user.id, recommended=True)
            add = len(player.queue) < MAX_PLAYLIST_LEN
            if add:
                player.add(requester=user.id, track=track)

                if not player.is_playing:
                    await player.play()

            return ['song', track, add]

    @commands.command(aliases=['p'])
    @doc_url("https://docs.archit.us/commands/music/#play")
    async def play(self, ctx, *, query: str):
        '''play <search item|url>
        Start playing a song or add songs to the queue.
        Supports youtube, spotify, twitch, and soundcloud.
        '''
        songs = await self.enqueue(query, ctx.author, ctx.guild)

        if len(songs) == 0:
            player = self.bot.lavalink.player_manager.get(ctx.guild.id)
            if not player.is_playing:
                await ctx.guild.change_voice_state(channel=None)
            return await ctx.send('no tracks found')

        if songs[0] == 'error':
            return await ctx.send(songs[1])

        embed = discord.Embed(color=discord.Color.blurple())
        if songs[0] == 'playlist':
            embed.title = 'Playlist Enqueued'
            embed.description = f'{songs[1]} - {songs[2]} tracks'
            if songs[3]:
                embed.description += f"\n{MAX_PLAYLIST_LEN} song limit reached"
        elif songs[0] == 'album':
            embed.title = 'Album Enqueued'
            embed.description = f'{songs[1]} - {songs[2]} tracks'
            if songs[3]:
                embed.description += f"\n{MAX_PLAYLIST_LEN} song limit reached"
        else:
            if songs[2]:
                track = songs[1]
                embed.title = 'Tracks enqueued'
                embed.description = f'[{track.title}]({track.uri})'
                if "youtube" in track.uri:
                    yt_id = track.uri.split("=")[1]
                    embed.set_thumbnail(url=f"https://img.youtube.com/vi/{yt_id}/1.jpg")
            else:
                embed.title = "Track limit reached"
                embed.description = f'Max of {MAX_PLAYLIST_LEN} songs can be in the queue'

        await ctx.send(embed=embed)

    @commands.command(aliases=['dc'])
    async def stop_music(self, ctx):
        '''stop_music or dc
        Will clear all music from the queue and disconnect architus from the vc.
        '''
        player = self.bot.lavalink.player_manager.get(ctx.guild.id)

        if not player.is_connected:
            return await ctx.send('not connected')

        if not ctx.author.voice or (player.is_connected and ctx.author.voice.channel.id != int(player.channel_id)):
            return await ctx.send('Not in my voice channel')

        player.queue.clear()
        await player.stop()
        await ctx.guild.change_voice_state(channel=None)
        await ctx.send('goodbye')


def setup(bot):
    bot.add_cog(LavaMusic(bot))
