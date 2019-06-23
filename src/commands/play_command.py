from src.commands.abstract_command import abstract_command
import youtube_dl
from discord.ext import commands
from src.guild_player import GuildPlayer
import re
import functools
import discord

class Play(commands.Cog):

    def __init__(self, bot):
        self.bot = bot
        self.players = {}

    @property
    def guild_settings(self):
        return self.bot.get_cog('GuildSettings')

    @commands.command()
    async def play(self, ctx, url):
        '''Add a song to the queue or play immediately. Supports youtube and spotify links.'''

        if ctx.guild not in self.players:
            self.players[ctx.guild] = GuildPlayer(self.bot)
        player = self.players[ctx.guild]
        settings = self.guild_settings.get_guild(ctx.guild)

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
                        await player.add_url_now(arg[1]);
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
            else:
                if (len(player.q) == 0):
                    message = "Play what, " + self.author.mention + "?"
                else:
                    name = await player.play()
                    if (name):
                        message = "🎶 **Now playing:** *%s*" % name

        await ctx.channel.send(message)

    @commands.command(aliases=['q'])
    async def queue(self, ctx):
        '''List songs in queue'''
        if ctx.guild not in self.players:
            self.players[ctx.guild] = GuildPlayer(self.bot)
        player = self.players[ctx.guild]
        settings = self.guild_settings.get_guild(ctx.guild)

        if not settings.music_enabled:
            return

        await ctx.channel.send(embed=player.qembed())

    @commands.command()
    async def skip(self, ctx):
        '''Skip a song'''
        if ctx.guild not in self.players:
            self.players[ctx.guild] = GuildPlayer(self.bot)
        print("hello?")
        player = self.players[ctx.guild]
        await player.skip()




    @commands.command()
    async def notplay(self, ctx, url):
        opts = {
            'format': 'webm[abr>0]/bestaudio/best',
            'prefer_ffmpeg': False
        }
        ydl = youtube_dl.YoutubeDL(opts)
        func = functools.partial(ydl.extract_info, url, download=False)
        info = await self.bot.loop.run_in_executor(None, func)
        if "entries" in info:
            info = info['entries'][0]

        download_url = info['url']
        vc = await ctx.author.voice.channel.connect()
        vc.play(discord.FFmpegPCMAudio(download_url), after=lambda e: print('done', e))
        await ctx.send(download_url)

def setup(bot):
    bot.add_cog(Play(bot))
