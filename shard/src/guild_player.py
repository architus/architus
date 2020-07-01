import youtube_dl
import functools
import discord
from collections import deque
import src.spotify_tools as spotify_tools
import urllib
import asyncio
import aiohttp
from bs4 import BeautifulSoup
from src.list_embed import ListEmbed as list_embed
from lib import config
from lib.config import logger

from os import mkfifo, unlink
from threading import Thread
import types
import pafy
import subprocess
# spotify_tools = None


class Ydl:
    def urlopen(self, url):
        return pafy.g.opener.open(url)

    def to_screen(self, *args, **kwargs):
        pass

    def to_console_title(self, *args, **kwargs):
        pass

    def trouble(self, *args, **kwargs):
        pass

    def report_warning(self, *args, **kwargs):
        pass

    def report_error(self, *args, **kwargs):
        pass


def callback(s):
    if s['status'] != 'downloading':
        return
    total = s['total_bytes']
    curr = s['downloaded_bytes']
    print("{downloaded_bytes:10} / {total_bytes:10}")


def download(self, filepath=""):
    downloader = youtube_dl.downloader.http.HttpFD(Ydl(),
                                                   {'nopart': True,
                                                    'noprogress': False,
                                                    'quiet': False,
                                                    'http_chunk_size': 1048576})
    infodict = {'url': self.url}
    downloader._progress_hooks = [callback]
    downloader.download(filepath, infodict)


def callback(*args):
    for a in args:
        print(a)


class GuildPlayer:
    def __init__(self, bot):
        logger.debug("creating new smart player")
        self.bot = bot
        self.q = deque()
        self.voice = None
        self.name = ''

        self.playing_file = False

        pafy.set_api_key(config.youtube_client_key)
        self.pipe = None

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
            if self.pipe is not None:
                unlink(self.pipe)
                self.pipe = None
            return ''
        if (self.voice and self.voice.is_playing()):
            if self.pipe is not None:
                unlink(self.pipe)
                self.pipe = None
            return await self.skip()
        self.stop()
        song = self.q.pop()
        url = song.url
        self.name = song.title
        logger.debug("starting " + url)
        if ('spotify' in url):
            url = await self.spotify_to_youtube(url)
            self.name = url['name']
            url = url['url']

        ffmpeg_options = {
            'options': '-vn',
        }

        audio = pafy.new(url)
        stream = None
        for s in audio.audiostreams:
            if s.bitrate == "128k":
                stream = s
                break

        if stream is None:
            stream = audio.getbestaudiostream()

        thing = types.MethodType(download, stream)
        self.pipe = "/tmp/" + audio.title + "." + stream.extension
        mkfifo(self.pipe)
        Thread(target=thing, kwargs={"filepath": self.pipe}).start()

        logger.debug(f"Named pipe: {self.pipe}")
        logger.debug(f"downloading song {stream.url}")
        logger.debug(f"Rate: {stream.bitrate}")
        # p = open(self.pipe, "rb")
        self.voice.play(discord.FFmpegPCMAudio(self.pipe, **ffmpeg_options), after=self.agane)

        logger.debug("Song playing has started")
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
        song = Song(url)
        self.q.appendleft(song)
        return song.title

    async def add_url_now(self, url):
        self.q.append(Song(url))

    async def get_youtube_url(self, search):
        '''scrape video url from search paramaters'''
        async with aiohttp.ClientSession() as session:
            query = urllib.parse.quote(search)
            url = "https://www.youtube.com/results?search_query=" + query
            async with session.get(url) as resp:
                html = await resp.read()

                soup = BeautifulSoup(html.decode('utf-8'), 'lxml')
                for video in soup.findAll(attrs={'class': 'yt-uix-tile-link'}):
                    if ('googleadservices' not in video['href']):
                        return 'https://www.youtube.com' + video['href']

        return ''

    def stop(self):
        if self.voice is None:
            return
        self.voice.stop()
        if self.pipe is not None:
            unlink(self.pipe)
            self.pipe = None

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
        return self.q[-1].title

    def clearq(self):
        self.q.clear()

    def qembed(self):
        name = self.name or "None"
        lem = list_embed("Currently Playing:", "*%s*" % name, self.bot.user)
        lem.color = 0x6600ff
        lem.icon_url = ''
        lem.name = "Song Queue"
        for i in range(len(self.q)):
            song = self.q[len(self.q) - i - 1]
            lem.add("%d. *%s*" % (i + 1, song.title), song.url)
        return lem.get_embed()

    def is_connected(self):
        return self.voice is not None and self.voice.is_connected()

    def agane(self, trash):
        if self.playing_file:
            return

        if len(self.q) == 0:
            if self.pipe is not None:
                unlink(self.pipe)
                self.pipe = None
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
    def __init__(self, url):
        self.url = url
        self._title = None
        self.spotify = 'track' in url

    @property
    def title(self):
        if self._title:
            return self._title

        if self.spotify:
            data = spotify_tools.generate_metadata(self.url)
            self._title = data['name'] if 'name' in data else 'n/a'
        else:
            ydl = youtube_dl.YoutubeDL({'outtmpl': '%(id)s%(ext)s'})

            with ydl:
                result = ydl.extract_info(self.url, download=False)
            video = result['entries'][0] if 'entries' in result else result

            self._title = video['title'] if 'title' in video else 'n/a'
        return self._title
