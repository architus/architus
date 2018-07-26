import os, random
#from queue import Queue
from collections import deque
#import src.spotify_tools
import urllib
#import urllib2
from bs4 import BeautifulSoup


MUSIC_PATH="/home/jbuchanan/Music/"
EXPLORATION_PATH=MUSIC_PATH+"Exploration"
TOWN_PATH=MUSIC_PATH+"Town"
BOSS_PATH=MUSIC_PATH+"Boss"
ENCOUNTER_PATH=MUSIC_PATH+"Encounter"

class smart_player:
    def __init__(self):
        self.q = deque()
        self.player = None
        self.voice = None
        self.setting = ''
        self.goAgane = False
        self.name = ''

    async def play(self):
        if (self.voice == None or len(self.q) == 0):
            return ''
        self.stop()
        self.player = await self.voice.create_ytdl_player(self.q.pop())
        print (self.player.error)
        #self.player.volume = 0.15
        self.player.start()
        return self.player.title


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

    def add_spotify_track(self, url):
        print(url)
        #data = spotify_tools.generate_metadata(url)
    def add_youtube_track(self, url):
        self.q.append(url)
    def add_youtube_track_now(self, url):
        self.q.appendleft(url)

    def get_youtube_url(self, search):
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
        self.stop()
        return await self.play()
    def clearq(self):
        self.q.clear()

    def is_connected(self):
        return self.voice != None and self.voice.is_connected()

    def agane(self):
        if (self.goAgane):
            self.play(self.setting)
