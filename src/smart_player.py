import os, random

MUSIC_PATH="/home/jbuchanan/Music/"
EXPLORATION_PATH=MUSIC_PATH+"Exploration"
TOWN_PATH=MUSIC_PATH+"Town"
BOSS_PATH=MUSIC_PATH+"Boss"
ENCOUNTER_PATH=MUSIC_PATH+"Encounter"

class smart_player:
    def __init__(self):
        self.player = None
        self.voice = None
        self.setting = ''
        self.goAgane = False
        self.name = ''

    def play(self, setting):
        if (self.voice == None):
            return
        self.goAgane = False
        self.stop()
        self.setting = setting
        del self.player
        if (setting.lower() == 'town'):
            self.player = self.play_town()
        elif (setting.lower() == 'exploration'):
            self.player = self.play_exploration()
        elif (setting.lower() == 'encounter'):
            self.player = self.play_encounter()
        elif (setting.lower() == 'boss'):
            self.player = self.play_boss()
        else:
            self.player = self.play_town()

        print('setting: ' + setting)
 
        self.player.volume = 0.15
        self.player.start()
        self.goAgane = True

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
    def skip(self):
        if (self.player == None):
            return
        self.stop()
        self.play(self.setting)

    def is_connected(self):
        return self.voice != None and self.voice.is_connected()

    def agane(self):
        if (self.goAgane):
            self.play(self.setting)
