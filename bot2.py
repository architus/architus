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

    async def fetch_user_dict(self, id):
        usr = await self.fetch_user(int(id))
        avatar = usr.avatar_url or usr.default_avatar_url
        return json.dumps({'name' : usr.name, 'avatar_url' : str(avatar)})


    @asyncio.coroutine
    def sub(self, ctx):
        #sub = ctx.socket(zmq.SUB)
        #sub.connect("tcp://127.0.0.1:7100")
        #sub.setsockopt(zmq.SUBSCRIBE, b"")
        #yield from sub.recv_multipart()
        #print("subbing")
        pub = ctx.socket(zmq.PUB)
        pub.bind("tcp://127.0.0.1:7208")
        while True:
            if hasattr(self, 'q') and not self.q.empty():
                #msg = yield from sub.recv_json()
                msg = json.loads(self.q.get())
                self.loop.create_task(self.handle_thing(pub, msg))
            yield from asyncio.sleep(.01)
            #yield from pub.send_json(b'2 hello')

    @asyncio.coroutine
    def handle_thing(self, pub, msg):
        try:
            resp = (yield from getattr(self, msg['method'])(msg['arg']))
        except Exception as e:
            print(f"caught {e} while handling {msg['topic']}s request")
            resp = '{"message": "' + str(e) + '"}'
        print("sending back " + str(resp))
        yield from pub.send((str(msg['topic']) + ' ' + str(resp)).encode())

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
coolbot.load_extension('src.commands.eight_ball_command')
coolbot.load_extension('src.commands.settings_command')
coolbot.load_extension('src.guild_settings')
ctx = zmq.asyncio.Context()
coolbot.loop.create_task(coolbot.sub(ctx))

if __name__ == '__main__':
    from src.config import secret_token
    coolbot.run(secret_token)

