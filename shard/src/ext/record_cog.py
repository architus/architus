"""
Cog for recording a voice channel
"""
from io import BytesIO
import base64
from collections import defaultdict
import asyncio
import time
from functools import partial
import secrets
import string
from concurrent.futures import ThreadPoolExecutor

import pyzipper
from src.utils import AsyncThreadEvent

import discord
from discord.ext import commands
from discord import WavFile


NUM_ZIP_THREADS = 5


class Recording:
    """
    Class to handle a recording for a guild.
    """

    def __init__(self, bot, ctx, budget, tp):
        self.bot = bot
        self.ctx = ctx
        self.budget = budget

        self.wav_file = None
        self.event = AsyncThreadEvent()

        self.recording = True
        self.sink = None

        self.thread_pool = tp

    def zip_file(self, password):
        zip_file = BytesIO()
        zipper = pyzipper.AESZipFile(zip_file,
                                     mode='w',
                                     compression=pyzipper.ZIP_DEFLATED,
                                     allowZip64=True,
                                     compresslevel=6,
                                     encryption=pyzipper.WZ_AES)
        zipper.setpassword(password)
        zipper.writestr("voice.wav", self.wav_file.getvalue())
        zipper.close()

        return zip_file

    async def send_file(self):
        try:
            self.ctx.voice_client.stop_listening()
            await self.event.wait()
        except Exception:
            await self.ctx.send("Something went wrong")
            await self.cleanup()
            return

        if len(self.wav_file.getvalue()) == 0:
            await self.ctx.send("Nothing was recorded")
            await self.cleanup()
            return

        loop = asyncio.get_running_loop()
        pw = bytes("".join([secrets.choice(string.ascii_letters + string.digits)
                            for _ in range(15)]), "ascii")
        zip_file = await loop.run_in_executor(self.thread_pool,
                                              self.zip_file,
                                              pw)
        self.wav_file = None

        url, _ = await self.bot.manager_client.publish_file(
            data=base64.b64encode(zip_file.getvalue()).decode('ascii'),
            filetype='zip',
            location='recordings')

        embed = discord.Embed(title="Voice Recording")
        embed.add_field(name="URL", value=url['url'])
        embed.add_field(name="Password", value=pw)
        embed.add_field(name="Channels", value=str(self.sink.num_channels))
        embed.add_field(name="Note", value="File will not work well in normal audio players."
                                           "Try using audacity to listen to the file.")
        await self.ctx.send(embed=embed)

    async def start_recording(self, excludes):
        """
        Records the voice channel that the commanding user is currently in.
        """
        try:
            await self.ensure_voice()
        except commands.CommandError:
            return 1

        members = [m for m in self.ctx.author.voice.channel.members
                   if not m.bot and m not in excludes]
        self.wav_file = BytesIO()
        self.sink = WavFile(self.wav_file, members, self.event, self.bot.user, excludes)
        self.ctx.voice_client.listen(self.sink)
        self.recording = True

        return 0

    async def stop_recording(self):
        """
        Stops the cog recording the voice channel and sends a link to the zip
        of the wav file.
        """

        if not self.recording:
            await self.ctx.send("Please start a reocrding first.")
            return

        self.recording = False
        num_bytes = len(self.wav_file.getvalue())

        async with self.ctx.typing():
            await self.send_file()

        await self.cleanup()
        return num_bytes

    async def timer(self):
        """
        Ensures that the recording does not go over a certain size.
        """
        while self.recording:
            if (len(self.sink.channels) > 0
                    and len(self.sink.channels[0]) * self.sink.num_channels * 3840 > self.budget):
                self.recording = False
                num_bytes = len(self.wav_file.getvalue())

                async with self.ctx.typing():
                    await self.send_file()

                await self.cleanup()
                return num_bytes
            await asyncio.sleep(15)
        return 0

    async def cleanup(self):
        """
        Remove bot from voice channel and clear per recording variables.
        """
        await self.ctx.voice_client.disconnect()
        self.event.clear()
        self.wav_file.close()
        self.wav_file = None

    async def ensure_voice(self):
        """
        Ensures various properties about the environment before the record
        method is called.
        """
        if self.ctx.author.voice:
            # Connect to author's voice channel before recording.
            await self.ctx.author.voice.channel.connect()
        else:
            await self.ctx.send("Need to be in a voice channel to record.")
            raise Exception


class RecordCog(commands.Cog, name="Voice Recording"):
    """
    Cog for recording voice channels.
    """

    def __init__(self, bot):
        self.bot = bot
        self.event = AsyncThreadEvent()

        self.budgets = defaultdict(lambda: 5000000000)
        self.recordings = defaultdict(lambda: None)
        self.last_recording = defaultdict(time.time)

        self.thread_pool = ThreadPoolExecutor(max_workers=NUM_ZIP_THREADS)

        # Ensure the opus library is loaded as it is needed to do voice things.
        if not discord.opus.is_loaded():
            discord.opus.load_opus("res/libopus.so")

    @commands.command(aliases=['record'])
    async def start_recording(self, ctx):
        """
        Records the voice channel that the commanding user is currently in.
        """
        guild_id = ctx.guild.id
        if self.recordings[guild_id] is not None:
            await ctx.send("I'm already recording. "
                           "Stop the current recording then start a new one.")
            return

        if (time.time() - self.last_recording[guild_id]) > 86400:
            self.budgets[guild_id] = 5000000000
            self.last_recording[guild_id] = time.time()

        excludes = [ctx.guild.get_member(uid)
                    for uid in self.bot.settings[ctx.guild].voice_exclude]
        recording = Recording(self.bot, ctx, self.budgets[guild_id], self.thread_pool)
        err = await recording.start_recording(excludes)

        if err == 0:
            self.recordings[guild_id] = recording
            task = asyncio.create_task(recording.timer())
            task.add_done_callback(partial(self.update, guild_id))

    @commands.command()
    async def stop_recording(self, ctx):
        """
        Stops the cog recording the voice channel.
        """

        guild_id = ctx.guild.id
        if self.recordings[guild_id] is None:
            await ctx.send("Please start a reocrding first.")
            return

        # Shouldn't need to set dict value to None as that will happen in the timer callback
        num_bytes = await self.recordings[guild_id].stop_recording()
        self.budgets[guild_id] -= num_bytes

    @commands.command(aliases=['delete'])
    async def delete_file(self, ctx, filename):
        """
        Delete specified voice recording from the cdn. Pass just the specific file name without
        directories.
        """
        result = await self.bot.manager_client.delete_file(filename)
        if result == 0:
            await ctx.send("File successfully deleted.")
        elif result == 1:
            await ctx.send("Tried to delete from invalid directory. How did you even do that?")
        elif result == 2:
            await ctx.send(f"{filename} does not exist.")

    @commands.command(aliases=['unmute_me'])
    async def mute_me(self, ctx):
        """
        Adds the user to a list of people that will not be recorded.
        Run again to toggle setting.
        """

        settings = self.bot.settings[ctx.guild]
        excludes = settings.voice_exclude
        author = ctx.author
        if author.id in excludes:
            excludes.remove(author.id)
            await ctx.send(f"{author.display_name} will now be included in voice recordings")
        else:
            excludes.append(author.id)
            await ctx.send(f"{author.display_name} will not be included in voice recordings")
        settings.voice_exclude = excludes

    def update(self, guild_id, fut):
        """
        Updates states of the recordings when a recording stops due to going over budget.
        Should only ever be called as a callback from the timer function in the Recording
        class.
        """
        self.recordings[guild_id] = None
        self.budgets[guild_id] -= fut.result()


def setup(bot):
    """
    Add cog to bot.
    """
    bot.add_cog(RecordCog(bot))
