# Cog for recording a voice channel

from discord.ext import commands
import discord
from discord import AudioSink

import zipfile
import asyncio
from io import BytesIO
from struct import pack
import base64


class AsyncThreadEvent(asyncio.Event):
    """
    Custom event class for being able to safely set and clear an
    asynchronous event from multiple threads.

    Shouldn't need a thread safe clear method as the event should only be
    referenced from the main thread by the time that that method is called.
    """

    def set(self):
        self._loop.call_soon_threadsafe(super().set)


class WavFile(AudioSink):
    """
    Custom wave file sink to write recorded voice to.

    The discord bot sends 3840 bytes of 16bit PCM data in each packet as soon as possible.
    This class then does its darndest to transform that live real time data into something
    more synchronized.

    The specific WAV file that this rights will have a couple of hard coded values.
    ChunkID: RIFF
    Format: WAVE
    SubChunk1ID: fmt (note: the space needs to be included in the header)
    AudioFormat: 1 (this specifies uncompressed PCM data)
    SampleRate: 48000 (this is the sample rate that discord uses (I think))
    BitsPerSample: 16 (this is definitely the number of bits per sample in the data
                       that discord sends to architus)
    SubChunk2ID: data
    """

    def __init__(self, f, user_list, event, bot_user):
        """
        :param: f Where to write the wave file to. This should just be a BytesIO object.
        :param: user_list List of users in the voice channel to include in recording
        """
        self.bot_user = bot_user
        self.event = event
        self.f = f
        self.user_list = user_list
        self.buffer = BytesIO()
        self.data_size = 0
        self.num_channels = len(user_list)
        self.channels = [list() for _ in range(self.num_channels)]
        self.packet_count = 0

        # this is equivalent to one packet of silence
        self.silence = b"\x00" * 3840

    def write(self, data):
        """
        This method will just properly store the data internally in the class
        so that it can be properly written out later with all of the proper
        header data.
        """

        # Check to see which channel the data should be written to.
        # Each user will get their own channel
        if data.user == self.bot_user:
            # I don't think the bot actually ever sends voice data,
            # but just to make sure.
            return
        channel = -1
        for i, u in enumerate(self.user_list):
            if u == data.user:
                channel = i
                break
        if channel == -1:
            # If user was not found, then add them and increase number of channels
            self.user_list.append(data.user)
            self.num_channels += 1
            self.channels.append([])

            # To keep audio roughly synced, add silence up until the current time.
            # The current time is taken as the channel with the most amount of
            # audio currently in it.
            longest = max([len(c) for c in self.channels])
            for _ in range(longest):
                self.channels[-1].append(self.silence)

        # The data comes in as two channel audio. Both channels have the same data
        # and we only need one channel so just take every other 16 bit chunk to
        # compress it down to a single channel.
        d = b"".join([data.data[i:i + 2] for i in range(0, len(data.data), 4)])
        self.channels[channel].append(d)
        self.packet_count += 1

        # roughly once a second, see if any of the channels is falling far behind the
        # others. If they are, this means that person has exited the channel and their
        # audio needs to be caught up with the rest of the channels.
        if self.packet_count > 10:
            self.packet_count = 0
            longest = max([len(c) for c in self.channels])
            for i in range(self.num_channels):
                if longest - len(self.channels[i]) > 5:
                    for _ in range(longest - len(self.channels[i])):
                        self.channels[i].append(self.silence)

    def cleanup(self):
        """
        Writing to WAV files is really weird. Sometimes the bytes need to be
        big endian and sometimes they need to be little endian. This weirdness is
        all in the header so it can be mostly just hard coded but needs to be paid
        special attention to.

        Most of my knowledge of how WAV files work comes from:
        http://soundfile.sapp.org/doc/WaveFormat/

        We just need to make sure that all of the header values match the specifics
        of the WAV file that we will be writing. The main thing we have to do on the
        fly is the number of channels, chunk sizes, block align, and byte rate.
        The actual data can mostly just be left alone apart from putting it in the
        right place.
        """

        wav_data = []
        size = max([len(c) for c in self.channels])
        for c in self.channels:
            if len(c) < size:
                c.append(b"\x00" * (size - len(c)))
            wav_data.append(b"".join(c))

        # Header values that can't be hardcoded
        data_chunk_size = self.num_channels * len(wav_data[0])
        chunk_size = pack("<L", 36 + data_chunk_size)
        data_chunk_size = pack("<L", data_chunk_size)
        byte_rate = pack("<L", 48000 * self.num_channels * 2)
        block_align = pack("<H", self.num_channels * 2)
        n_chan = pack("<H", self.num_channels)

        # RIFF chunk descriptor
        self.f.write(b"\x52\x49\x46\x46")              # "RIFF", specifies RIFF file type
        self.f.write(chunk_size)                       # Size of the file minus this and "RIFF"
        self.f.write(b"\x57\x41\x56\x45")              # "WAVE", specifies wave subtype

        # fmt sub chunk
        self.f.write(b"\x66\x6d\x74\x20")              # "fmt ", starts format section
        self.f.write(b"\x10\x00\x00\x00")              # 16, size of this part of header
        self.f.write(b"\x01\x00")                      # 1, PCM mode
        self.f.write(n_chan)                           # number of channels
        self.f.write(b"\x80\xBB\x00\x00")              # 48000, sample rate of file
        self.f.write(byte_rate)                        # byte rate
        self.f.write(block_align)                      # number of bytes in an entire sample of all channels
        self.f.write(b"\x10\x00")                      # Bits in a sample of one channel

        # data chunk
        self.f.write(b"\x64\x61\x74\x61")              # "data", in data header now
        self.f.write(data_chunk_size)                  # size of the data chunk

        # write the actual PCM data
        for j in range(0, len(wav_data[0]), 2):
            for i in range(self.num_channels):
                self.f.write(wav_data[i][j:j + 2])     # make sure to write two bytes as sample size is 16 bits

        self.event.set()


class RecordCog(commands.Cog):
    def __init__(self, bot):
        self.bot = bot
        self.recording = False
        self.wav_file = None
        self.event = AsyncThreadEvent()

        # Ensure the opus library is loaded as it is needed to do voice things.
        if not discord.opus.is_loaded():
            discord.opus.load_opus("res/libopus.so")

    @commands.command(alias=['record'])
    async def start_recording(self, ctx):
        """
        Records the voice channel that the commanding user is currently in.
        """
        if self.recording:
            await ctx.send("I'm already recording. Stop the current recording then start a new one.")
            return

        try:
            await self.ensure_voice(ctx)
        except commands.CommandError:
            return

        self.recording = True

        members = [m for m in ctx.author.voice.channel.members if not m.bot]
        self.wav_file = BytesIO()
        sink = WavFile(self.wav_file, members, self.event, self.bot.user)
        ctx.voice_client.listen(sink)

    @commands.command()
    async def stop_recording(self, ctx):
        if not self.recording:
            await ctx.send("Please start a reocrding first.")
            return

        try:
            ctx.voice_client.stop_listening()
            await self.event.wait()
        except Exception:
            await ctx.send("Something went wrong")
            await self.cleanup(ctx)

        zip_file = BytesIO()
        zipper = zipfile.ZipFile(zip_file, mode='w', compression=zipfile.ZIP_DEFLATED,
                                 allowZip64=True, compresslevel=6)
        zipper.writestr("voice.wav", self.wav_file.getvalue())
        zipper.close()

        url, _ = await self.bot.manager_client.publish_file(
            data=base64.b64encode(zip_file.getvalue()).decode('ascii'),
            filetype='zip',
            location='recordings')

        await ctx.send(f"You can find your voice recording here {url['url']}")
        await self.cleanup(ctx)

    async def cleanup(self, ctx):
        await ctx.voice_client.disconnect()
        self.event.clear()
        self.recording = False
        self.wav_file.close()
        self.wav_file = None

    async def ensure_voice(self, ctx):
        """
        Ensures various properties about the environment before the record
        method is called.
        """
        if ctx.author.voice:
            # Connect to author's voice channel before recording.
            await ctx.author.voice.channel.connect()
            return True
        else:
            await ctx.send("Need to be in a voice channel to record.")


def setup(bot):
    bot.add_cog(RecordCog(bot))
