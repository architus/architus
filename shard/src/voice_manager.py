from discord import VoiceChannel, opus, FFmpegPCMAudio, Embed, Guild
import youtube_dlc as youtube_dl

from typing import List, Optional, Tuple
from random import choice
from functools import partial
import asyncio
from datetime import datetime

from lib.config import logger
from src import spotify_tools
from src.utils import format_seconds


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
    'max_filesize': 52428800,
    'source_address': '0.0.0.0'  # bind to ipv4 since ipv6 addresses cause issues sometimes
}


class Song:
    def __init__(self, url: str, name: str, download_url: str, filename: str, duration: int):
        self.url = url
        self.name = name
        self.download_url = download_url
        self.filename = filename
        self.duration = duration

    @classmethod
    async def from_youtube(cls, search: str, no_playlist: bool = True, retries: int = 0) -> List['Song']:
        opts = ytdl_opts.copy()
        opts.update({'noplaylist': no_playlist})
        ydl = youtube_dl.YoutubeDL(opts)
        f = partial(ydl.extract_info, search, download=True)
        loop = asyncio.get_running_loop()
        logger.debug(f"searching yt for {search}")
        try:
            data = await loop.run_in_executor(None, f)
        except (youtube_dl.utils.ExtractorError, youtube_dl.utils.DownloadError):
            logger.exception(f"error downloading video data... ({retries + 1}/4)")
            if retries > 2:
                return []
            return await cls.from_youtube(search, retries=retries + 1)
        logger.debug(f"num of entries: {len(data['entries'])}")
        try:
            return [
                cls(e['webpage_url'], e['title'], e['url'], ydl.prepare_filename(e), e['duration'])
                for e in data['entries']]
        except KeyError:
            logger.exception(data)
            return []

    @classmethod
    async def from_spotify(cls, spotify_url: str) -> List['Song']:
        loop = asyncio.get_running_loop()
        if '/track/' in spotify_url:
            urls = [spotify_url]
        elif '/playlist' in spotify_url:
            f = partial(spotify_tools.fetch_spotify_playlist_songs, spotify_url)
            name, urls = await loop.run_in_executor(None, f)

        async def to_yt(url):
            try:
                search = await loop.run_in_executor(None, partial(spotify_tools.spotify_to_str, url))
            except Exception:
                logger.exception("")
                pass
            return await cls.from_youtube(search)
        tasks = [asyncio.create_task(to_yt(url)) for url in urls]
        return [song for t in tasks for song in await t]

    def __repr__(self):
        return f"[{self.name}]({self.url})"


class SongQueue:
    def __init__(self, bot, guild: Guild):
        self.bot = bot
        self.guild = guild
        self.q = []  # type: List[Song]
        self.now_playing = None  # type: Optional[Song]
        self.started_at = None  # type: Optional[datetime]
        self.shuffle = False  # type: bool

    @property
    def position(self) -> Optional[Tuple[int, int]]:
        if self.started_at is None or self.now_playing is None:
            return None
        return int((datetime.now() - self.started_at).total_seconds()), self.now_playing.duration

    def insert_l(self, songs: List[Song]) -> None:
        for s in songs:
            self.push(s)

    def insert(self, i: int, song: Song) -> None:
        if song is None:
            return
        self.q.insert(i, song)

    def push(self, song: Song) -> None:
        self.insert(0, song)

    def pop(self) -> Song:
        if len(self.q) < 1:
            raise IndexError("song queue is empty")
        if self.shuffle:
            self.now_playing = choice(self.q)
            self.q.remove(self.now_playing)
            return self.now_playing
        else:
            self.now_playing = self.q[-1]
            return self.q.pop()

    def __len__(self):
        return len(self.q)

    def __getitem__(self, key):
        return self.q[key]

    def __delitem__(self, key):
        del self.q[key]

    def embed(self):
        songs = "\n".join(f"**{i + 1:>2}.** *{song}*" for i, song in enumerate(self.q[::-1]))
        i = 0
        while len(songs) > 2000:
            # shorten embed to fit within discord's limits
            songs = "\n".join(f"**{i + 1:>2}.** *{song}*" for i, song in enumerate(self.q[:i:-1]))
            songs += f"\n*{i + 1} more not shown...*"
            i += 1
        if self.now_playing is not None:
            long = self.now_playing.duration > 3600
            title = self.now_playing.name
            url = self.now_playing.url
        else:
            title = "no songs queued"
            url = None
        name = ""
        if self.position:
            name = f"Now Playing ({format_seconds(self.position[0], long)}/{format_seconds(self.position[1], long)}):"
        else:
            name = "Now Playing:"
        em = Embed(title=title, url=url, description=songs, color=0x6600ff)
        em.set_author(name=name)
        em.set_footer(text=f"🔀 Shuffle: {self.shuffle}")
        return em


class VoiceManager:
    def __init__(self, bot, guild: Guild):
        self.bot = bot
        self.guild = guild
        self.voice = None
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
            song = self.q.pop()

        self.voice.play(FFmpegPCMAudio(song.download_url, **ffmpeg_options), after=self._finalizer)
        self.q.started_at = datetime.now()
        return song

    def _finalizer(self, error):
        self.q.started_at = None
        if error is not None:
            logger.error(error)
        if len(self.q) < 1:
            coro = self.disconnect()
            self.q.now_playing = None
        else:
            coro = self.play()
        asyncio.run_coroutine_threadsafe(coro, self.bot.loop)
