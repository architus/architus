import os
import random
import string
import asyncio
from gtts import gTTS
import discord


class say_command():

    def __init__(self):
        super().__init__("say")

    async def exec_cmd(self, **kwargs):
        if len(self.args) > 1:
            players = kwargs['players']
            player = players[self.server.id]
            if not discord.opus.is_loaded():
                discord.opus.load_opus('res/libopus.so')
            if not (player.is_connected()):
                # TODO
                voice = await self.author.voice.channel.connect()
                player.voice = voice
            else:
                player.voice.move_to(self.author.voice.voice_channel)

            tts = gTTS(text=' '.join(self.args[1:]), lang='en')
            key = ''.join([random.choice(string.ascii_letters) for n in range(10)])
            async with self.channel.typing():
                tts.save("res/generate/%s.mp3" % key)
                voice.play(discord.FFmpegPCMAudio("res/generate/{}.mp3".format(key)))
                # done = await player.play_file("res/generate/%s.mp3" % key)
            await asyncio.sleep(10)
            os.remove("res/generate/%s.mp3" % key)
            return True


def setup(bot):
    pass
