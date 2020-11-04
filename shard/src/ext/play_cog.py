from discord.ext import commands
from typing import Dict, List
from src.guild_player import GuildPlayer
from src.voice_manager import VoiceManager, Song
from src.utils import doc_url
from lib.config import logger
import discord
import asyncio


class VoiceManagers(dict):
    def __init__(self, bot):
        self.bot = bot
        super().__init__()

    def __missing__(self, key):
        if key not in self:
            guild = self.bot.get_guild(key)
            if not guild:
                raise KeyError(f"No guild found for id: {key}")
            self[key] = VoiceManager(self.bot, self.bot.get_guild(key))
        return self[key]


class VoiceCog(commands.Cog, name="Voice"):
    def __init__(self, bot):
        self.bot = bot
        self.voice_managers = VoiceManagers(bot)  # type: Dict[int, VoiceManager]

    @commands.command()
    async def test(self, ctx):
        m = self.voice_managers[ctx.guild.id]
        while m.linebuffer:
            await ctx.send(m.linebuffer.pop(0))


    @commands.Cog.listener()
    async def on_guild_join(self, guild):
        self.voice_managers[guild.id] = VoiceManager(self.bot, guild)

    async def enqueue(self, ctx, query: str) -> List[Song]:
        manager = self.voice_managers[ctx.guild.id]
        try:
            if ('/playlist/' in query or '/track/' in query):
                songs = await Song.from_spotify(query)
            else:
                songs = await Song.from_youtube(query)
        except Exception as e:
            logger.exception("")
            await ctx.send(f"Error queuing song {e}")
            return ()
        else:
            manager.q.insert_l(songs)

        if len(songs) > 1:
            await ctx.send(f"*Successfully queued {len(songs)} songs*")
        elif len(songs) == 1:
            await ctx.send(f"Successfully queued *{songs[0].name}*")
        return songs

    @commands.command(aliases=['p'])
    @doc_url("https://docs.archit.us/commands/music/#play")
    async def play(self, ctx, *song: str):
        '''play <search item|youtube url|spotify url>
        Add a song to the music queue.
        Supports youtube and spotify links.
        '''
        manager = self.voice_managers[ctx.guild.id]
        settings = self.bot.settings[ctx.guild]
        if ctx.author.voice and (ch := ctx.author.voice.channel):
            if manager.channel and ch.id != manager.channel.id:
                if ctx.author.id not in settings.admin_ids:
                    ctx.send("Please join a voice channel to use music commands")
                    return
            await manager.join(ch)
        else:
            ctx.send("Please join a voice channel to use music commands")
            return

        arg = " ".join(song)
        async with ctx.channel.typing():
            if len(await self.enqueue(ctx, arg)) < 1:
                return

            if manager.is_playing:
                pass
            else:
                try:
                    song = await manager.play()
                except Exception as e:
                    logger.exception("")
                    await ctx.send(f"error playing music {e}")
                else:
                    msg = song.name if 'youtu' in arg else song.url
                    await ctx.send(f"now playing: {msg}")

    @commands.group(aliases=["q"])
    @doc_url("https://docs.archit.us/commands/")
    async def queue(self, ctx):
        '''queue [add|rm|show|{song}] args'''
        if ctx.invoked_subcommand is None:
            await ctx.send(embed=self.voice_managers[ctx.guild.id].q.embed())

    @queue.command(aliases=['a'])
    async def add(self, ctx, *query):
        '''queue add <songs>'''
        await self.enqueue(ctx, " ".join(query))

    @queue.command(aliases=['remove', 'r'])
    async def rm(self, ctx, index: int):
        '''queue rm <index>'''
        manager = self.voice_managers[ctx.guild.id]
        try:
            song = manager.q[index - 1]
            del manager.q[index - 1]
        except IndexError:
            await ctx.send("not sure what song you're talking about")
        else:
            await ctx.send(f"removed *{song.name}*")

    @queue.command()
    async def shuffle(self, ctx):
        '''queue shuffle'''
        q = self.voice_managers[ctx.guild.id].q
        q.shuffle = not q.shuffle
        await ctx.send(f"Shuffle is **{'on' if q.shuffle else 'off'}**")


class BPlay(commands.Cog, name="Music Player"):
    '''
    Can join voice and play beautiful noises
    '''

    def __init__(self, bot):
        self.bot = bot
        self.players = {}

    @commands.command(aliases=['p'])
    @doc_url("https://docs.archit.us/commands/music/#play")
    async def play(self, ctx, url):
        '''play <search item|youtube url|spotify url>
        Add a song to the music queue.
        Supports youtube and spotify links.
        '''

        if ctx.guild not in self.players:
            self.players[ctx.guild] = GuildPlayer(self.bot)
        player = self.players[ctx.guild]
        settings = self.bot.settings[ctx.guild]

        if not settings.music_enabled:
            return True

        async with ctx.channel.typing():
            if not discord.opus.is_loaded():
                discord.opus.load_opus('res/libopus.so')
            if not (player.is_connected()):
                voice = await ctx.author.voice.channel.connect()
                player.voice = voice
            else:
                await player.voice.move_to(ctx.author.voice.channel)

            arg = ctx.message.content.split(' ')
            add = arg[0] != '!playnow' and (player.q or (player.voice and player.voice.is_playing()))
            message = ''
            if (len(arg) > 1):
                try:
                    if ('/playlist/' in arg[1]):
                        urls = await player.add_spotify_playlist(arg[1])
                        message = "Queuing \"" + urls[0] + "\"."
                        del urls[0]
                        await player.add_url(urls[0])
                        name = await player.play()
                        for track in urls:
                            await player.add_url(track)
                        if (name):
                            message += "\n🎶 **Playing:** *%s*" % name
                    elif ('/track/' in arg[1]):
                        if (add):
                            name = await player.add_url(arg[1])
                            message = '**Queued:** *%s*' % name
                        else:
                            await player.add_url_now(arg[1])
                            name = await player.play()
                            if (name):
                                message = "🎶 **Now playing:** *%s*" % name
                    elif ('youtu' in arg[1]):
                        if (add):
                            name = await player.add_url(arg[1])
                            message = '**Queued:** *%s*' % name
                        else:
                            player.pause()
                            await player.add_url_now(arg[1])
                            name = await player.play()
                            if (name):
                                message = "🎶 **Playing:** *%s*" % name
                    else:
                        del arg[0]
                        url = await player.get_youtube_url(' '.join(arg))
                        if (add):
                            await player.add_url(url)
                            message = "**Queued:** *%s*" % url
                        else:
                            await player.add_url_now(url)
                            name = await player.play()
                            if (name):
                                message = "🎶 **Now Playing:** " + url
                except Exception:
                    logger.exception("error queuing song")
                    message = f"❌ error queuing"
            else:
                if (len(player.q) == 0):
                    message = "Play what, " + self.author.mention + "?"
                else:
                    name = await player.play()
                    if (name):
                        message = "🎶 **Now playing:** *%s*" % name

        await ctx.channel.send(message)

    @commands.command(aliases=['q'])
    @doc_url("https://docs.archit.us/commands/music/#queue")
    async def queue(self, ctx):
        '''queue
        List songs in queue.'''
        if ctx.guild not in self.players:
            self.players[ctx.guild] = GuildPlayer(self.bot)
        player = self.players[ctx.guild]
        settings = self.bot.settings[ctx.guild]

        if not settings.music_enabled:
            return

        await ctx.channel.send(embed=await player.qembed())

    @commands.command()
    @doc_url("https://docs.archit.us/commands/music/#skip")
    async def skip(self, ctx):
        '''skip
        Skip a song.'''
        if ctx.guild not in self.players:
            self.players[ctx.guild] = GuildPlayer(self.bot)
        player = self.players[ctx.guild]
        name = await player.skip()
        if name:
            await ctx.channel.send(f"🎶 **Now playing:** *{name}*")
        else:
            await ctx.channel.send("No songs left. goodbye")

    @commands.command()
    @doc_url("https://docs.archit.us/commands/music/#clear")
    async def clear(self, ctx):
        '''clear
        Clear all songs from queue.'''
        if ctx.guild not in self.players:
            self.players[ctx.guild] = GuildPlayer(self.bot)
        player = self.players[ctx.guild]
        await ctx.channel.send("Removed %d songs from queue." % len(player.q))
        player.clearq()


def setup(bot):
    bot.add_cog(VoiceCog(bot))