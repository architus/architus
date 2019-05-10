import os, random
import youtube_dl
from threading import Thread
import multiprocessing as mp

import discord
#from queue import Queue
from collections import deque
import src.spotify_tools as spotify_tools
import urllib
import asyncio
import aiohttp
#import urllib2
from bs4 import BeautifulSoup
from src.list_embed import list_embed



class smart_player:
    def __init__(self, client):
        self.client = client
        self.q = deque()
        self.player = None
        self.voice = None
        self.name = ''

        self.playing_file = False

    async def play_file(self, filepath):
        self.playing_file = True
        self.stop()
        if (self.voice == None):
            return
        #if (self.player and self.player.is_playing()):
            #return await self.skip()

        try:
            self.player = self.voice.create_ffmpeg_player(filepath, after=self.agane)
            self.player.start();
        except Exception as e:
            print(e)
        self.playing_file = False
        return True

    async def play(self):
        if (self.voice == None or len(self.q) == 0):
            return ''
        if (self.player and self.player.is_playing()):
            return await self.skip()
        self.stop()
        self.name = ''
        url = self.q.pop().url
        print("starting " + url)
        if ('spotify' in url):
            url = await self.spotify_to_youtube(url)
            self.name = url['name']
            url = url['url']
        try:
            options = "-reconnect 1 -reconnect_streamed 1 -reconnect_delay_max 5"
            self.player = await self.voice.create_ytdl_player(url, before_options=options, ytdl_options={'--prefer-insecure'}, after=self.agane)
            self.player.volume = 0.30
            self.player.start()
            if not self.name:
                self.name = self.player.title
            return self.name
        except Exception as e:
            print("couldn't create player")
            print(e)
            if (self.player):
                print(self.player.error)
        return ''

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
                except KeyError: pass
            if tracks['next']:
                tracks = spotify.next(tracks)
            else:
                break

        return urls

    async def spotify_to_youtube(self, url):
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
        async with aiohttp.ClientSession() as session:
            query = urllib.parse.quote(search)
            url = "https://www.youtube.com/results?search_query=" + query
            async with session.get(url) as resp:
                html = await resp.read()
                def get_video(html):
                    soup = BeautifulSoup(html, 'lxml')
                    print (soup.findAll(attrs={'class':'yt-uix-tile-link'}, limit=2))
                    for video in soup.findAll(attrs={'class':'yt-uix-tile-link'}):
                        if ('googleadservices' not in video['href']):
                            return 'https://www.youtube.com' + video['href']
                loop = asyncio.get_event_loop()
                return await loop.run_in_executor(None, get_video, html)

        return ''

    def stop(self):
        if (self.player == None):
            return
        self.player.stop()

    def pause(self):
        if (self.player == None):
            return
        self.player.pause()
    def resume(self):
        if (self.player == None):
            return
        self.player.resume()
    async def skip(self):
        if (self.player == None):
            return
        old_name = self.name
        if (len(self.q) < 1):
            print("len was less than 1")
            await self.voice.disconnect()
            return ''
        self.stop()
        #while (old_name == self.name):
        count = 0
        while (not self.name or self.name == old_name) and count < 20:
            await asyncio.sleep(.5)
            print(self.name)
            count += 1
        return self.name
    #return await self.play()
    def clearq(self):
        self.q.clear()

    def qembed(self):
        name = self.name or "None"
        lem = list_embed("Currently Playing:", "*%s*" % name, self.client.user)
        lem.color = 0x6600ff
        lem.icon_url = ''
        lem.name = "Song Queue"
        for i in range(len(self.q)):
            song = self.q[len(self.q) - i - 1]
            lem.add("%d. *%s*" % (i + 1, song.title), song.url)
        return lem.get_embed()

    def is_connected(self):
        return self.voice != None and self.voice.is_connected()

    def agane(self):
        #coro = self.client.send_message(self.client.get_channel('436189230390050830'), 'Song is done!')
        if self.playing_file:
            return

        if len(self.q) == 0:
            coro = self.voice.disconnect()
        else:
            coro = self.play()
        fut = discord.compat.run_coroutine_threadsafe(coro, self.client.loop)
        try:
            fut.result()
        except:
            print('error')
            # an error happened sending the message
            pass
        #await self.play()

class Song:
    def __init__(self, url):
        self.url = url
        self._title = None
        self.spotify = 'track' in url

    @property
    def title(self):
        if self._title: return self._title

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
