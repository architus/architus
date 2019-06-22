from src.commands.abstract_command import abstract_command
import youtube_dl
from discord.ext import commands
import re
import functools
import discord

class play_command(abstract_command):

    def __init__(self):
        super().__init__("play", aliases=['notplay'])

    async def exec_cmd(self, **kwargs):

        players = kwargs['players']
        settings = kwargs['settings']

        if not settings.music_enabled:
            return True

        player = players[self.server.id]
        async with self.channel.typing():
            if not discord.opus.is_loaded():
                discord.opus.load_opus('res/libopus.so')
            if not (player.is_connected()):
                voice = await self.client.join_voice_channel(self.author.voice.voice_channel)
                player.voice = voice
            else:
                player.voice.move_to(self.author.voice.voice_channel)

            arg = self.content.split(' ')
            add = arg[0] != '!playnow' and (player.q or (player.player and player.player.is_playing()))
            message = ''
            if (len(arg) > 1):
                if ('/playlist/' in arg[1]):
                    try:
                        urls = await player.add_spotify_playlist(arg[1])
                        message = "Queuing \"" + urls[0] + "\"."
                        del urls[0]
                        await player.add_url(urls[0])
                        name = await player.play()
                        for track in urls:
                            await player.add_url(track)
                        if (name):
                            message += "\n🎶 **Playing:** *%s*" % name
                    except:
                        message = "something went badly wrong please spam my creator with pings"
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
                elif ('town' in arg[1] or 'encounter' in arg[1] or 'boss' in arg[1] or 'exploration' in arg[1]):
                    message = "Please pass in the url of the playlist."
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



        #print ('q ' + str(len(player.q)))
        #for song in list(player.q):
            #print ('song: ' + song)

        await self.channel.send(message)
        
        return True
        

    def get_help(self, **kwargs):
        return "Add a song to the queue or play immediately. Supports youtube and spotify links."

class PlayCommand(commands.Cog):

    def __init__(self, bot):
        self.bot = bot

    @commands.command()
    async def play(self, ctx, url):
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
    bot.add_cog(PlayCommand(bot))
