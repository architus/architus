import youtube_dlc as youtube_dl
import functools
import discord
from collections import deque
import src.spotify_tools as spotify_tools
import asyncio
from src.list_embed import ListEmbed as list_embed
from lib.config import logger

import subprocess
# spotify_tools = None


class GuildPlayer:
    def __init__(self, bot):
        logger.debug("creating new guild player")
        self.bot = bot
        self.q = deque()
        self.voice = None
        self.name = ''

        self.playing_file = False

    async def play_file(self, filepath):
        self.playing_file = True
        self.stop()
        if self.voice is None:
            return
        # if (self.player and self.player.is_playing()):
            # return await self.skip()

        try:
            self.player = self.voice.create_ffmpeg_player(filepath, after=self.agane, stderr=subprocess.STDOUT)
            self.player.start()
        except Exception:
            logger.exception("can't create new player")
        self.playing_file = False
        return True

    async def play(self):
        if (self.voice is None or len(self.q) == 0):
            return ''
        if (self.voice and self.voice.is_playing()):
            return await self.skip()
        self.stop()
        song = self.q.pop()
        url = song.url
        self.name = await song.title()
        logger.debug("starting " + url)
        if ('spotify' in url):
            url = await self.spotify_to_youtube(url)
            self.name = url['name']
            url = url['url']

        opts = {
            'prefer_ffmpeg': True,
            'format': 'bestaudio/best',
            'outtmpl': '%(extractor)s-%(id)s-%(title)s.%(ext)s',
            'restrictfilenames': True,
            'noplaylist': True,
            'nocheckcertificate': True,
            'ignoreerrors': False,
            'logtostderr': False,
            'quiet': True,
            'no_warnings': True,
            'default_search': 'auto',
            'source_address': '0.0.0.0'  # bind to ipv4 since ipv6 addresses cause issues sometimes
        }
        ffmpeg_options = {
            'options': '-vn -reconnect 1 -reconnect_streamed 1 -reconnect_delay_max 5'
        }
        ydl = youtube_dl.YoutubeDL(opts)
        func = functools.partial(ydl.extract_info, url, download=True)
        info = await self.bot.loop.run_in_executor(None, func)
        if "entries" in info:
            info = info['entries'][0]

        # download_url = info['url']
        download_url = ydl.prepare_filename(info)
        logger.debug(f"downloading url {download_url}")
        self.voice.play(discord.FFmpegPCMAudio(download_url, **ffmpeg_options), after=self.agane)
        # await asyncio.sleep(2)
        # os.remove(download_url)
        return self.name

    async def add_spotify_playlist(self, url):
        urls = []
        data = spotify_tools.fetch_playlist(url)
        urls.append(data['name'])
        tracks = data['tracks']
        while True:
            for item in tracks['items']:
                if 'track' in item:
                    track = item['track']
                else:
                    track = item
                try:
                    track_url = track['external_urls']['spotify']
                    urls.append(track_url)
                except KeyError:
                    pass
            if tracks['next']:
                # TODO
                # tracks = spotify.next(tracks)
                pass
            else:
                break

        return urls

    async def spotify_to_youtube(self, url):
        '''convert spotify track to youtube url by searching for 'song - artist'''
        info = {}
        data = spotify_tools.generate_metadata(url)
        info['name'] = data['name']
        info['url'] = await self.get_youtube_url("%s - %s" % (info['name'], data['artists'][0]['name']))
        return info

    async def add_url(self, url):
        song = Song(self.bot.loop, url)
        self.q.appendleft(song)
        return await song.title()

    async def add_url_now(self, url):
        self.q.append(Song(self.bot.loop, url))

    async def get_youtube_url(self, search):
        '''scrape video url from search paramaters'''
        opts = {
            'format': 'bestaudio/best',
            'ignoreerrors': False,
            'logtostderr': False,
            'quiet': True,
            'no_warnings': True,
            'default_search': 'auto',
            'noplaylist': True
        }

        ydl = youtube_dl.YoutubeDL(opts)
        f = functools.partial(ydl.extract_info, search, download=False)
        data = await self.bot.loop.run_in_executor(None, f)
        if 'entries' in data:
            data = data['entries'][0]
        return data['webpage_url']

    def stop(self):
        if self.voice is None:
            return
        self.voice.stop()

    def pause(self):
        if self.voice is None:
            return
        self.voice.pause()

    def resume(self):
        if self.voice is None:
            return
        self.voice.resume()

    async def skip(self):
        if self.voice is None:
            logger.debug("no voice")
            return
        if (len(self.q) < 1):
            logger.debug("len(q) < 1")
            await self.voice.disconnect()
            return ''
        self.stop()
        return await self.q[-1].title()

    def clearq(self):
        self.q.clear()

    async def qembed(self):
        name = self.name or "None"
        lem = list_embed("Currently Playing:", "*%s*" % name, self.bot.user)
        lem.color = 0x6600ff
        lem.icon_url = ''
        lem.name = "Song Queue"
        for i in range(len(self.q)):
            song = self.q[len(self.q) - i - 1]
            lem.add("%d. *%s*" % (i + 1, await song.title()), song.url)
        return lem.get_embed()

    def is_connected(self):
        return self.voice is not None and self.voice.is_connected()

    def agane(self, trash):
        if self.playing_file:
            return

        if len(self.q) == 0:
            coro = self.voice.disconnect()
        else:
            coro = self.play()
        fut = asyncio.run_coroutine_threadsafe(coro, self.bot.loop)
        try:
            fut.result()
        except Exception:
            logger.exception('error playing next thing')


class Song:
    '''Represents a spotify or youtube url'''
    def __init__(self, loop, url):
        self.loop = loop
        self.url = url
        self._title = None
        self.spotify = 'track' in url

    async def title(self):
        if self._title:
            return self._title

        def fetch_title():
            try:
                if self.spotify:
                    data = spotify_tools.generate_metadata(self.url)
                    return data['name'] if 'name' in data else 'n/a'
                else:
                    ydl = youtube_dl.YoutubeDL({'outtmpl': '%(id)s%(ext)s'})

                    with ydl:
                        result = ydl.extract_info(self.url, download=False)
                    video = result['entries'][0] if 'entries' in result else result

                    return video['title'] if 'title' in video else 'n/a'
            except Exception:
                logger.exception("error fetching title")
                return "<error fetching title>"
        self._title = await self.loop.run_in_executor(None, fetch_title)
        return self._title
