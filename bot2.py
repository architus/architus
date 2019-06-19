import discord
from discord.ext import commands
from discord.ext.commands import Bot
import asyncio
import zmq
import zmq.asyncio
import json

from multiprocessing import Pipe


class CoolBot(Bot):

    @commands.command()
    async def test(ctx):
        print(ctx.message.content)

    @asyncio.coroutine
    def sub(self, ctx):
        #sub = ctx.socket(zmq.SUB)
        #sub.connect("tcp://127.0.0.1:7100")
        #sub.setsockopt(zmq.SUBSCRIBE, b"")
        #yield from sub.recv_multipart()
        #print("subbing")
        pub = ctx.socket(zmq.PUB)
        pub.bind("tcp://127.0.0.1:7208")
        print("thing running")
        while True:
            if hasattr(self, 'q') and not self.q.empty():
                #msg = yield from sub.recv_json()
                msg = json.loads(self.q.get())
                print("recievede something, creating task")
                self.loop.create_task(self.handle_thing(pub, msg))
            yield from asyncio.sleep(.01)
            #yield from pub.send_json(b'2 hello')

    @asyncio.coroutine
    def handle_thing(self, pub, msg):
        try:
            resp = yield from getattr(self, msg['method'])(msg['arg']).name
        except Exception as e:
            resp = e
        print("sending back " + str(resp))
        yield from pub.send((str(msg['topic']) + ' ' + resp).encode())


    async def fill_q(self):
        while True:
            if self and hasattr(self, 'q'):
                if self.q.qsize() < 4:
                    print("adding new pipe")
                    here, there = Pipe()
                    self.q.put(there)
                    self.loop.create_task(self.poll_pipe(here))
            await asyncio.sleep(.01)

    async def poll_pipe(self, conn):
        while True:
            #channel = self.get_channel(436189230390050830)
            if conn.poll():
                call = json.loads(conn.recv())
                try:
                    resp = await getattr(self, call['method'])(call['arg'])
                except Exception as e:
                    print(e)
                    conn.send('idk')
                else:
                    conn.send(resp.name)
                conn.close()
                return
            await asyncio.sleep(.01)

    async def on_ready(self):
        self.add_command(self.test)
        print('Logged on as {0}!'.format(self.user))
        await self.change_presence(activity=discord.Activity(name="the tragedy of darth plagueis the wise", type=3))

    async def on_message(self, message):
        await self.process_commands(message)
        print('Message from {0.author}: {0.content}'.format(message))

BOT_PREFIX = ("?", "!")
coolbot = CoolBot(command_prefix=BOT_PREFIX)

coolbot.load_extension('src.commands.schedule_command')
ctx = zmq.asyncio.Context()
coolbot.loop.create_task(coolbot.sub(ctx))

