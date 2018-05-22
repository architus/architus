import random
import asyncio
import aiohttp
import discord
import time
from pytz import timezone
import pytz
from discord import Game
from discord import message
from discord.ext.commands import Bot

import config

BOT_PREFIX = ("?", "!")
TOKEN = config.secret_token

PECHS_ID = '178700066091958273'
JOHNYS_ID = '214037134477230080'

client = Bot(command_prefix=BOT_PREFIX)

@client.command(name='8ball',
                description="Answers a yes/no question.",
                brief="Answers from the beyond.",
                aliases=['eight_ball', 'eightball', '8-ball'],
                pass_context=True)
async def eight_ball(context):
    possible_responses = [
        'That is a resounding no',
        'It is not looking likely',
        'Too hard to tell',
        'It is quite possible',
        'Definitely',
    ]
    await client.say(random.choice(possible_responses) + ", " + context.message.author.mention)

@client.event
async def on_message_delete(message):
    if (message.author.id == PECHS_ID):
        time_posted = message.timestamp
        time_posted_utc = time_posted.replace(tzinfo=timezone('UTC'))
        time_posted_est = time_posted_utc.astimezone(timezone('US/Eastern'))
        em = discord.Embed(title=time_posted_est.strftime("%Y-%m-%d %I:%M %p"), description=message.content, colour=0xffff00)
        em.set_author(name=message.author.display_name, icon_url=message.author.avatar_url)
        await client.send_message(message.channel, embed=em)

@client.event
async def on_message_edit(before, after):
    if (before.author.id == PECHS_ID):
        time_posted = after.timestamp
        time_posted_utc = time_posted.replace(tzinfo=timezone('UTC'))
        time_posted_est = time_posted_utc.astimezone(timezone('US/Eastern'))
        em = discord.Embed(title=time_posted_est.strftime("%Y-%m-%d %I:%M %p"), description='"'+before.content + '" ➡ "' + after.content+'"', colour=0xffff00)
        em.set_author(name=after.author.display_name, icon_url=before.author.avatar_url)
        await client.send_message(before.channel, embed=em)



@client.command()
async def square(number):
    squared_value = int(number) * int(number)
    await client.say(str(number) + " squared is " + str(squared_value))

def is_me(m):
    return m.author == client.user

@client.command(name='purge',
                description="Deletes the bot's spam.",
                brief="Delete spam.",
                aliases=[],
                pass_context=True)
async def purge(context):
    if (context.message.author.id != PECHS_ID):
        deleted = await client.purge_from(context.message.channel, limit=100, check=is_me)
        await client.send_message(context.message.channel, 'Deleted {} message(s)'.format(len(deleted)))



@client.event
async def on_ready():
    await client.change_presence(game=Game(name="PECH IS BOOSTED"))
    print("Logged in as " + client.user.name)


async def list_servers():
    await client.wait_until_ready()
    while not client.is_closed:
        print("Current servers:")
        for server in client.servers:
            print(server.name)
        await asyncio.sleep(600)


client.loop.create_task(list_servers())
client.run(TOKEN)
