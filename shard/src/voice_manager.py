from discord import VoiceChannel, opus, FFmpegPCMAudio, Embed
import youtube_dlc as youtube_dl

from typing import List, Optional
from random import choice
from functools import partial
import asyncio

from lib.config import logger
from src.list_embed import ListEmbed as list_embed


ffmpeg_options = {
    'options': '-vn -reconnect 1 -reconnect_streamed 1 -reconnect_delay_max 5'
}

ytdl_opts = {
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


class Song:
    def __init__(self, url: str, name: str, download_url: str):
        self.url = url
        self.name = name
        self.download_url = download_url

    @classmethod
    async def from_youtube(cls, search: str, retries: int = 0) -> Optional['Song']:
        ydl = youtube_dl.YoutubeDL(ytdl_opts)
        f = partial(ydl.extract_info, search, download=False)
        loop = asyncio.get_running_loop()
        try:
            data = await loop.run_in_executor(None, f)
        except (youtube_dl.utils.ExtractorError, youtube_dl.utils.DownloadError):
            logger.exception(f"error downloading video data... ({retries + 1}/4)")
            if retries > 2:
                return
            return await cls.from_youtube(search, retries=retries + 1)
        if data['_type'] == 'playlist':
            try:
                data = data['entries'][0]
            except IndexError:
                return None
        try:
            return cls(data['webpage_url'], data['title'], data['url'])
        except KeyError:
            logger.exception("")
            return None

    @classmethod
    async def from_spotify(cls, spotify_url: str) -> List['Song']:
        # TODO
        return []

    def __repr__(self):
        return f"[{self.name}]({self.url})"


class SongQueue:
    def __init__(self, bot, guild):
        self.bot = bot
        self.guild = guild
        self.q = []
        self.now_playing = None

    def insert_l(self, songs: List[Song]) -> None:
        for s in songs:
            self.push(s)

    def insert(self, i: int, song: Song) -> None:
        if song is None:
            return
        self.q.insert(i, song)

    def push(self, song: Song) -> None:
        self.insert(0, song)

    def pop(self, shuffle=False) -> Song:
        if len(self.q) < 1:
            raise IndexError("song queue is empty")
        if shuffle:
            self.now_playing = choice(self.q)
            self.q.remove(self.now_playing)
            return self.now_playing
        else:
            self.now_playing = self.q[-1]
            return self.q.pop()

    def __len__(self):
        return len(self.q)

    def embed(self):
        songs = "\n".join(f"{i}. *{song}*" for i, song in enumerate(self.q))
        em = Embed(title=self.now_playing.name, url=self.now_playing.url, description=songs, color=0x6600ff)
        em.set_author(name="Now Playing:")
        em.add_field(name="Shuffle", value="*off*")
        return em


class VoiceManager:
    def __init__(self, bot, guild):
        self.bot = bot
        self.guild = guild
        self.voice = None
        self.shuffle = False
        self.q = SongQueue(bot, guild)

        if not opus.is_loaded():
            opus.load_opus('res/libopus.so')

    @property
    def channel(self):
        return self.voice.channel if self.voice else None

    @property
    def is_playing(self):
        return self.voice is not None and self.voice.is_playing()

    async def join(self, ch: VoiceChannel):
        if self.channel is None:
            self.voice = await ch.connect()
        elif self.channel.id == ch.id:
            pass
        else:
            self.voice.move_to(ch)

    async def disconnect(self):
        if self.voice is not None:
            await self.voice.disconnect()

    async def play(self, song: Song = None):
        if self.channel is None or self.voice is None:
            raise
        if self.is_playing:
            self.voice.stop()
        if song is None:
            song = self.q.pop(self.shuffle)

        self.voice.play(FFmpegPCMAudio(song.download_url, **ffmpeg_options), after=self._finalizer)
        return song

    def _finalizer(self, error):
        if error is not None:
            logger.error(error)
        if len(self.q) < 1:
            coro = self.disconnect()
            self.q.now_playing = None
        else:
            coro = self.play(self.q.pop(self.shuffle))
        asyncio.run_coroutine_threadsafe(coro, self.bot.loop)
