"""
Cog for recording a voice channel
"""
from collections import defaultdict
import asyncio
import time
from functools import partial
import socket
import struct
from concurrent.futures import ThreadPoolExecutor

from src.utils import TCPLock

import discord
from discord import TCPSink
from discord.ext import commands


class Recording:
    """
    Class to handle a recording for a guild.
    """

    def __init__(self, bot, ctx, budget):
        self.bot_id = bot
        self.ctx = ctx
        self.budget = budget

        self.wav_file = None

        self.recording = True
        self.sink = None
        self.tcp = None

    async def send_file(self):
        """
        try:
            self.ctx.voice_client.stop_listening()
        except Exception:
            await self.ctx.send("Something went wrong")
            await self.cleanup()
            return

        loop = asyncio.get_running_loop()
        url, _ = await self.bot.manager_client.publish_file(
            data=base64.b64encode(zip_file.getvalue()).decode('ascii'),
            filetype='zip',
            location='recordings')

        embed = discord.Embed(title=f"Recording of {self.vc.name}", url=url['url'])
        embed.add_field(name="URL", value=url['url'], inline=False)
        embed.add_field(name="Password", value=pw)
        if self.sink.num_channels > 3:
            embed.add_field(name="Channels", value=str(self.sink.num_channels))
        embed.set_footer(text="File will not work well in normal audio players."
                              "Try using audacity to listen to the file.")
        await self.ctx.send(embed=embed)
        """

        msg = self.tcp.recv(8)
        size = "I" if len(msg) == 4 else "Q"
        num_bytes = struct.unpack(size, msg)[0]
        await self.ctx.send(f"Recorded {num_bytes} bytes")
        return num_bytes

    def send_disallowed_ids(self, ids):
        buf = bytearray(4096)
        sent = 0
        to_send = len(ids)
        while to_send > 0:
            sending = min(to_send, 64)
            struct.pack_into(f">{sending}Q", buf, 0, *members[sent:sent + sending])
            self.tcp.send(buf)
            to_send -= sending

    async def start_recording(self, excludes):
        """
        Records the voice channel that the commanding user is currently in.
        """
        try:
            await self.ensure_voice()
        except commands.CommandError:
            return 1

        members = [m.id for m in self.ctx.author.voice.channel.members
                   if not m.bot and m not in excludes]

        self.tcp = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.tcp.connect(("record-service", 7777))
        to_send = len(members)
        buf = bytearray(11)
        struct.pack_into(">BQH", buf, 0, 0x03, self.bot_id, to_send)
        self.tcp.send(buf)
        print(f"Sending {to_send} user_ids")

        self.tcp = TCPLock(self.tcp)
        self.sink = TCPSink(self.tcp)
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
        self.ctx.voice_client.stop_listening()
        print("Sending kill signal")
        self.tcp.send(b"\x04")
        print("kill sent")

        async with self.ctx.typing():
            num_bytes = await self.send_file()

        await self.cleanup()
        return num_bytes

    def get_curr_recording_length(self):
        self.tcp.write(b"\x05")
        msg = self.tcp.recv(8)
        size = "I" if len(msg) == 4 else "Q"
        num_bytes = struct.unpack(size, msg)
        return num_bytes[0]

    async def timer(self):
        """
        Ensures that the recording does not go over a certain size.
        """
        loop = asyncio.get_running_loop()
        while self.recording:
            """
            with ThreadPoolExecutor() as pool:
                num_bytes = await loop.run_in_executor(
                    pool, self.get_curr_recording_length)
            """
            num_bytes = self.get_curr_recording_length()
            if (num_bytes > self.budget):
                self.recording = False
                self.ctx.voice_client.stop_listening()
                self.tcp.send(b"\x05")

                num_bytes = 0
                async with self.ctx.typing():
                    num_bytes = await self.send_file()

                await self.cleanup()
                return num_bytes
            await asyncio.sleep(15)
        return 0

    async def cleanup(self):
        """
        Remove bot from voice channel and clear per recording variables.
        """
        await self.ctx.voice_client.disconnect()

    async def ensure_voice(self):
        """
        Ensures various properties about the environment before the record
        method is called.
        """
        if self.ctx.author.voice:
            # Connect to author's voice channel before recording.
            await self.ctx.author.voice.channel.connect()
            self.channel = self.ctx.author.voice.channel
        else:
            await self.ctx.send("Need to be in a voice channel to record.")
            raise Exception


class RecordCog(commands.Cog, name="Voice Recording"):
    """
    Cog for recording voice channels.
    """

    def __init__(self, bot):
        self.bot = bot

        self.budgets = defaultdict(lambda: 5000000000)
        self.recordings = defaultdict(lambda: None)
        self.last_recording = defaultdict(time.time)

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
        recording = Recording(self.bot.user.id, ctx, self.budgets[guild_id])
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
