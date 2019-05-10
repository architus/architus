from src.commands.abstract_command import abstract_command
import os, random, string, asyncio
from gtts import gTTS
import discord

class say_command(abstract_command):

    def __init__(self):
        super().__init__("say")

    async def exec_cmd(self, **kwargs):
        if len(self.args) > 1:
            players = kwargs['players']
            player = players[self.server.id]
            if not discord.opus.is_loaded():
                discord.opus.load_opus('res/libopus.so')
            if not (player.is_connected()):
                voice = await self.client.join_voice_channel(self.author.voice.voice_channel)
                player.voice = voice
            else:
                player.voice.move_to(self.author.voice.voice_channel)


            tts = gTTS(text=' '.join(self.args[1:]), lang='en')
            key = ''.join([random.choice(string.ascii_letters) for n in range(10)])
            tts.save("res/generate/%s.mp3" % key)
            done = await player.play_file("res/generate/%s.mp3" % key)
            await self.client.send_typing(self.channel)
            await asyncio.sleep(10)
            os.remove("res/generate/%s.mp3" % key)

    def get_help(self):
        return "Say a message in voice channel"
    def get_usage(self):
        return "<message>"
