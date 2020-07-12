"""
Cog for recording a voice channel
"""
from collections import defaultdict
import asyncio
import time
from functools import partial
import socket
import struct

from src.utils import TCPLock
from lib.ipc import grpc_client
from lib.ipc import feature_gate_pb2 as message

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

        self.checkup = bytearray(4096)
        self.checkup[0] = 5

    async def send_file(self):
        while True:
            try:
                msg = self.tcp.recv(1024)
                break
            except BlockingIOError:
                await asyncio.sleep(1)
            except Exception:
                await self.ctx.send("Connection to recording service failed")
                return 0
        if msg[0] != 0x00:
            if msg[0] == 0x01:
                await self.ctx.send("Nothing was recorded")
            elif msg[0] == 0x02:
                await self.ctx.send("Failed to write data to WAV file")
            elif msg[0] == 0x03:
                await self.ctx.send("Failed to zip WAV file")
            elif msg[0] == 0x04:
                await self.ctx.send("Recording was too large for WAV file")
            elif msg[0] == 0x05:
                await self.ctx.send("Too many users in recording")
            elif msg[0] == 0x06:
                await self.ctx.send("Byte rate of recording was too large")
            elif msg[0] == 0x07:
                await self.ctx.send("Failed to upload file to CDN")
            elif msg[0] == 0x08:
                await self.ctx.send("Rust's async library failed")
            elif msg[0] == 0x09:
                await self.ctx.send("Failed to make temporary directory")
            else:
                self.ctx.send("Recieved unknown error from client")
            return 0

        url_len = struct.unpack(">Q", msg[1:9])[0]
        url = msg[9:9 + url_len].decode('ascii')
        curr = 9 + url_len
        pw_len = struct.unpack(">Q", msg[curr:curr + 8])[0]
        curr += 8
        pw = msg[curr:curr + pw_len].decode('ascii')
        print(f"Password: {pw}")
        curr += pw_len
        channels = struct.unpack(">Q", msg[-16:-8])[0]
        num_bytes = struct.unpack(">Q", msg[-8:])[0]

        embed = discord.Embed(title="Voice Recording", description="Get your fun sounds here!", url=url)
        embed.add_field(name="URL", value=url)
        embed.add_field(name="Password", value=pw)
        if channels > 3:
            embed.add_field(name="# of Channels", value=str(channels))
            embed.set_footer(text="Note: Audio will not play correctly in most programs. "
                                  "Try using audacity to listen to the file.")
        await self.ctx.send(embed=embed)
        return num_bytes

    def send_disallowed_ids(self, ids):
        buf = bytearray(4096)
        sent = 0
        to_send = len(ids)
        while to_send > 0:
            sending = min(to_send, 64)
            struct.pack_into(f">{sending}Q", buf, 0, *ids[sent:sent + sending])
            self.tcp.send(buf)
            to_send -= sending

    async def start_recording(self, excludes):
        """
        Records the voice channel that the commanding user is currently in.
        """
        try:
            await self.ensure_voice()
        except Exception:
            return 1

        self.tcp = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
        self.tcp.connect(("record", 7777))
        to_send = len(excludes)
        buf = bytearray(4096)
        struct.pack_into(">BQH", buf, 0, 0x03, self.bot_id, to_send)
        self.tcp.send(buf)
        self.send_disallowed_ids(excludes)

        self.tcp.setblocking(0)
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
        self.tcp.send(b"\x04")
        await asyncio.sleep(3)

        async with self.ctx.typing():
            num_bytes = await self.send_file()

        await self.cleanup()
        return num_bytes

    async def get_curr_recording_length(self):
        """
        Sends the checkup code to the microservice and then reads
        back how many bytes have been recorded so far.
        """
        self.tcp.write(self.checkup)
        await asyncio.sleep(1)
        msg = self.tcp.recv(8)

        # if didn't receive anything, return 0 and try again later
        if len(msg) < 8:
            return 0
        num_bytes = struct.unpack(">Q", msg)[0]
        return num_bytes

    async def timer(self):
        """
        Ensures that the recording does not go over a certain size.
        """
        await asyncio.sleep(15)
        while self.recording:
            try:
                num_bytes = await self.get_curr_recording_length()
            except Exception:
                await self.ctx.send("Lost TCP connection to recording microservice")
                return 0
            if (num_bytes > self.budget or num_bytes > 4000000000):
                self.recording = False
                self.ctx.voice_client.stop_listening()
                try:
                    self.tcp.send(b"\x04")
                except Exception:
                    await self.ctx.send("Lost TCP connection to recording microservice")
                    return 0
                await asyncio.sleep(3)

                num_bytes = 0
                async with self.ctx.typing():
                    await self.ctx.send("Used up all of alloted memory. Sending current recording.")
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

    async def feature_check(self, guildID):
        """
        Check with the feature server to determine whether or not the 'Voice'
        feature is activated for this guild.
        """
        m = message.GuildFeature(guild_id=guildID, feature_name="Voice")
        feat = await self.bot.feature_client.CheckGuildFeature(m)
        return feat.has_feature

    @commands.command(aliases=['record'])
    async def start_recording(self, ctx):
        """
        Records the voice channel that the commanding user is currently in.
        """
        guild_id = ctx.guild.id
        if not await self.feature_check(guild_id):
            await ctx.send("The voice recording feature has not been activated for this server.")
            return

        if self.recordings[guild_id] is not None:
            await ctx.send("I'm already recording. "
                           "Stop the current recording then start a new one.")
            return

        if (time.time() - self.last_recording[guild_id]) > 86400:
            self.budgets[guild_id] = 5000000000
            self.last_recording[guild_id] = time.time()

        excludes = self.bot.settings[ctx.guild].voice_exclude
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

    """ Need to temporarily disable this
    @commands.command(aliases=['delete'])
    async def delete_file(self, ctx, filename):
        # Delete specified voice recording from the cdn. Pass just the specific file name without
        # directories.
        result = await self.bot.manager_client.delete_file(filename)
        if result == 0:
            await ctx.send("File successfully deleted.")
        elif result == 1:
            await ctx.send("Tried to delete from invalid directory. How did you even do that?")
        elif result == 2:
            await ctx.send(f"{filename} does not exist.")
    """

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
