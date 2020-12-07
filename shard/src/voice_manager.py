from discord import VoiceChannel, opus, FFmpegPCMAudio, PCMVolumeTransformer, Embed, Guild, errors
import youtube_dlc as youtube_dl

from typing import List, Optional, Tuple
from random import choice
from functools import partial
import asyncio
import os
from datetime import datetime

from lib.config import logger
from src import spotify_tools
from src.utils import format_seconds


ffmpeg_options = {
    'options': '-vn -reconnect 1 -reconnect_streamed 1 -reconnect_delay_max 5 -af loudnorm=I=-16:TP=-1.5:LRA=11',
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
    def __init__(
            self,
            url: str,
            name: str,
            download_url: str,
            filename: str,
            duration: int,
            thumbnail_url: str):
        self.url = url
        self.name = name
        self.download_url = download_url
        self.filename = filename
        self.duration = duration
        self.thumbnail_url = thumbnail_url

    def as_embed(self) -> Embed:
        em = Embed(
            title=self.name,
            url=self.url,
            color=0xFF0000
        )
        em.set_thumbnail(url=self.thumbnail_url)
        em.set_author(name="Youtube", url="https://youtube.com")
        em.set_footer(text=format_seconds(self.duration, self.duration > 3600))
        return em

    def as_dict(self):
        return {
            'url': self.url,
            'name': self.name,
            'duration': self.duration,
            'thumbnail_url': self.thumbnail_url,
        }

    @classmethod
    async def from_youtube(cls, search: str, no_playlist: bool = True, retries: int = 0) -> List['Song']:
        """takes a youtube url of a song or playlist or a search query"""
        opts = ytdl_opts.copy()
        opts.update({'noplaylist': no_playlist})
        ydl = youtube_dl.YoutubeDL(opts)
        f = partial(ydl.extract_info, search, download=True)
        loop = asyncio.get_running_loop()
        logger.debug(f"searching yt for {search}")
        try:
            data = await loop.run_in_executor(None, f)
        except (youtube_dl.utils.ExtractorError, youtube_dl.utils.DownloadError):
            logger.debug(f"error downloading video data... ({retries + 1}/4)")
            if retries > 2:
                return []
            return await cls.from_youtube(search, retries=retries + 1)

        entries = data['entries'] if 'entries' in data else [data]
        try:
            return [
                cls(e['webpage_url'], e['title'], e['url'], ydl.prepare_filename(e), e['duration'], e['thumbnail'])
                for e in entries]
        except KeyError:
            logger.exception(data)
            return []

    @classmethod
    async def from_spotify(cls, spotify_url: str) -> List['Song']:
        """takes spotify track url or playlist url"""
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

    def __eq__(self, song):
        if isinstance(song, self.__class__):
            # this is the only equality that matters for deleting old songs
            return self.filename == song.filename
        return False


class SongQueue:
    def __init__(self, bot, guild: Guild):
        self.bot = bot
        self.guild = guild
        self.q = []  # type: List[Song]
        self.now_playing = None  # type: Optional[Song]
        self.started_at = None  # type: Optional[datetime]
        self.shuffle = False  # type: bool

    def as_dict(self):
        return {
            'pos': self.position,
            'now_playing': self.now_playing.as_dict(),
            'shuffle': self.shuffle,
            'songs': [s.as_dict() for s in self.q],
        }

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

    def clear(self) -> int:
        n = len(self.q)
        self.q = []
        self.now_playing = None
        return n

    def __len__(self):
        return len(self.q)

    def __getitem__(self, key):
        return self.q[key]

    def __delitem__(self, key):
        del self.q[key]

    def __contains__(self, key):
        return key in self.q

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
        self.garbage = []  # type: List[Song]

        if not opus.is_loaded():
            opus.load_opus('res/libopus.so')

    @property
    def channel(self):
        return self.voice.channel if self.voice else None

    @property
    def is_playing(self):
        return self.voice is not None and self.voice.is_playing()

    @property
    def settings(self):
        return self.bot.settings[self.guild]

    async def empty_garbage(self):
        loop = asyncio.get_running_loop()

        def empty():
            for song in self.garbage:
                if song in self.q:
                    continue
                if os.path.exists(song.filename):
                    os.remove(song.filename)
                self.garbage.remove(song)
        logger.debug(f"garbage contains {len(self.garbage)} items, emptying...")
        await loop.run_in_executor(None, empty)

    async def join(self, ch: VoiceChannel):
        if self.channel is None:
            self.voice = await ch.connect()
        elif self.channel.id == ch.id:
            pass
        else:
            self.voice.move_to(ch)

    async def disconnect(self, force=False):
        if self.channel is not None:
            await self.voice.disconnect(force=force)
            self.voice = None

    async def skip(self) -> Optional[Song]:
        if not self.voice:
            return
        # check if this is the last song before we stop cause race conditions
        last = len(self.q) == 0
        then_playing = self.q.now_playing
        # stopping the player will trigger the finalizer and automatically queue the next track
        self.voice.stop()
        if last:
            raise IndexError("no more songs")
        for _ in range(50):
            # yield until the song changes
            if self.q.now_playing and self.q.now_playing is not then_playing:
                return self.q.now_playing
            await asyncio.sleep(0)
        return None

    async def play(self, song: Song = None):
        if self.channel is None or self.voice is None:
            raise
        if self.is_playing:
            self.voice.stop()
        if song is None:
            song = self.q.pop()
        if not os.path.exists(song.filename):
            raise Exception("There was a problem downloading the song (probably too large)")
        try:
            self.voice.play(PCMVolumeTransformer(
                FFmpegPCMAudio(song.filename, **ffmpeg_options),
                self.settings.music_volume), after=self._finalizer)
        except errors.ClientException:
            logger.exception("hello")
            ch = self.channel
            logger.debug(ch)
            await self.disconnect(force=True)
            await self.join(ch)
            return await self.play(song=song)
        self.q.started_at = datetime.now()
        return song

    def _finalizer(self, error):
        if self.q.now_playing:
            self.garbage.append(self.q.now_playing)
        self.q.started_at = None
        if error is not None:
            logger.error(error)
        if len(self.q) < 1:
            coro = self.disconnect()
            self.q.now_playing = None
        else:
            coro = self.play()
        asyncio.run_coroutine_threadsafe(coro, self.bot.loop)

        if len(self.garbage) > 10:
            asyncio.create_task(self.empty_garbage())
