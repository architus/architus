import os, random
import discord
#from queue import Queue
from collections import deque
import src.spotify_tools as spotify_tools
import urllib
import asyncio
#import urllib2
from bs4 import BeautifulSoup


MUSIC_PATH="/home/jbuchanan/Music/"
EXPLORATION_PATH=MUSIC_PATH+"Exploration"
TOWN_PATH=MUSIC_PATH+"Town"
BOSS_PATH=MUSIC_PATH+"Boss"
ENCOUNTER_PATH=MUSIC_PATH+"Encounter"

class smart_player:
    def __init__(self, client):
        self.client = client
        self.q = deque()
        self.player = None
        self.voice = None
        self.setting = ''
        self.goAgane = False
        self.name = ''

    async def play(self, isManual=True):
        if (self.voice == None or len(self.q) == 0):
            return ''
        if (self.player and self.player.is_playing()):
            return await self.skip()
        self.stop()
        self.name = ''
        url = self.q.pop()
        if ('spotify' in url):
            url = await self.spotify_to_youtube(url)
            self.name = url['name']
            url = url['url']
        print("starting " + url)
        try:
            self.player = await self.voice.create_ytdl_player(url, after=self.agane)
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
        #self.player.volume = 0.15


    #def play(self, setting):
    #    if (self.voice == None):
    #        return
    #    self.goAgane = False
    #    self.stop()
    #    self.setting = setting
    #    del self.player
    #    if (setting.lower() == 'town'):
    #        self.player = self.play_town()
    #    elif (setting.lower() == 'exploration'):
    #        self.player = self.play_exploration()
    #    elif (setting.lower() == 'encounter'):
    #        self.player = self.play_encounter()
    #    elif (setting.lower() == 'boss'):
    #        self.player = self.play_boss()
    #    else:
    #        self.player = self.play_town()

    #    print('setting: ' + setting)
 
    #    self.player.volume = 0.15
    #    self.player.start()
    #    self.goAgane = True
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
                    #log.debug(track_url)
                    #print(track_url)
                    urls.append(track_url)
                    #self.add_spotify_track(track_url)
                    #track_urls.append(track_url)
                except KeyError:
                    pass
                    #log.warning(u'Skipping track {0} by {1} (local only?)'.format(
                    #track['name'], track['artists'][0]['name']))
            # 1 page = 50 results
            # check if there are more pages
            if tracks['next']:
                tracks = spotify.next(tracks)
            else:
                break

        return urls
        #utubeurl = self.get_youtube_url("%s - %s" % (name, data['artists'][0]['name']))
        #self.add_youtube_track(utubeurl)
        #return name

    async def spotify_to_youtube(self, url):
        info = {}
        data = spotify_tools.generate_metadata(url)
        info['name'] = data['name']
        info['url'] = await self.get_youtube_url("%s - %s" % (info['name'], data['artists'][0]['name']))
        return info

    async def add_url(self, url):
        self.q.append(url)

    async def add_url_now(self, url):
        self.q.appendleft(url)

    async def get_youtube_url(self, search):
        query = urllib.parse.quote(search)
        url = "https://www.youtube.com/results?search_query=" + query
        response = urllib.request.urlopen(url)
        html = response.read()
        soup = BeautifulSoup(html, 'lxml')
        for video in soup.findAll(attrs={'class':'yt-uix-tile-link'}):
            if ('googleadservices' not in video['href']):
                return 'https://www.youtube.com' + video['href']
        return ''
        

    def play_town(self):
        song = random.choice(os.listdir(TOWN_PATH))
        print(song)
        self.name = song[:-4]
        return self.voice.create_ffmpeg_player(TOWN_PATH + "/" + song, after=self.agane)

    def play_exploration(self):
        song = random.choice(os.listdir(EXPLORATION_PATH))
        print(song)
        self.name = song[:-4]
        return self.voice.create_ffmpeg_player(EXPLORATION_PATH + '/' + song, after=self.agane)

    def play_boss(self):
        song = random.choice(os.listdir(BOSS_PATH))
        print(song)
        self.name = song[:-4]
        return self.voice.create_ffmpeg_player(BOSS_PATH + '/' + song, after=self.agane)

    def play_encounter(self):
        song = random.choice(os.listdir(ENCOUNTER_PATH))
        print(song)
        self.name = song[:-4]
        return self.voice.create_ffmpeg_player(ENCOUNTER_PATH + '/' + song, after=self.agane)

    def stop(self):
        if (self.player == None):
            return
        self.player.stop()
        if (len(self.q) < 1):
            self.voice.disconnect()

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
        self.stop()
        if (len(self.q) < 1):
            self.voice.disconnect()
            return ''
        #while (old_name == self.name):
        await asyncio.sleep(2)
        return self.name
        #return await self.play()
    def clearq(self):
        self.q.clear()

    def is_connected(self):
        return self.voice != None and self.voice.is_connected()

    def agane(self):
        if (len(self.q) < 1):
            self.voice.disconnect()
        #coro = self.client.send_message(self.client.get_channel('436189230390050830'), 'Song is done!')
        coro = self.play()
        fut = discord.compat.run_coroutine_threadsafe(coro, self.client.loop)
        try:
            fut.result()
        except:
            print('error')
            # an error happened sending the message
            pass
        #await self.play()
