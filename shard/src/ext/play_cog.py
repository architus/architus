from discord.ext import commands
from src.guild_player import GuildPlayer
from src.voice_manager import VoiceManager, Song
from src.utils import doc_url
from lib.config import logger
import discord


class VoiceCog(commands.Cog, name="Voice"):
    def __init__(self, bot):
        self.bot = bot
        self.players = {}

    @commands.Cog.listener()
    async def on_ready(self):
        self.voice_managers = {g.id: VoiceManager(self.bot, g) for g in self.bot.guilds}

    @commands.Cog.listener()
    async def on_guild_join(self, guild):
        self.voice_managers[guild.id] = VoiceManager(self.bot, guild)

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
            if ch.id != manager.channel.id:
                if ctx.author.id not in settings.admin_ids:
                    ctx.send("Please join a voice channel to use music commands")
                    return
            await manager.join(ch)
        else:
            ctx.send("Please join a voice channel to use music commands")
            return

        arg = "".join(song)

        try:
            if ('/playlist/' in arg or '/track/' in arg):
                songs = Song.from_spotify(arg)
            else:
                songs = [Song.from_youtube(arg)]
        except Exception as e:
            logger.exception("")
            await ctx.send(f"error queuing music {e}")
        else:
            manager.q.insert_l(songs)
        if self.manager.is_playing:
            msg = '\n'.join(s.name for s in songs)
            await ctx.send(f"successuflly queued: {msg}")
        else:
            try:
                song = await manager.play()
            except Exception as e:
                logger.exception("")
                await ctx.send(f"error queuing music {e}")
            else:
                msg = song.name if 'youtu' in arg else song.url
                await ctx.send(f"now playing: {msg}")


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
