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
import spectrum_gen

BOT_PREFIX = ("?", "!")
TOKEN = config.secret_token

PECHS_ID = '178700066091958273'
JOHNYS_ID = '214037134477230080'

client = Bot(command_prefix=BOT_PREFIX)

karma_dict = {}

AUT_EMOJI = "🅱️"
NORM_EMOJI = "reee"
NICE_EMOJI = "❤"
TOXIC_EMOJI = "pech"

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

@client.event
async def on_reaction_add(reaction, user):
    author = reaction.message.author
    if (author != user and user != client.user):
        if (author not in karma_dict):
            karma_dict[author] = [0,0,0,0]
        if (str(reaction.emoji) == AUT_EMOJI or (reaction.custom_emoji and reaction.emoji.name == AUT_EMOJI)):
            karma_dict[author][0] += 1
        elif (str(reaction.emoji) == NORM_EMOJI or (reaction.custom_emoji and reaction.emoji.name == NORM_EMOJI)):
            karma_dict[author][1] += 1
        elif (str(reaction.emoji) == NICE_EMOJI or (reaction.custom_emoji and reaction.emoji.name == NICE_EMOJI)):
            karma_dict[author][2] += 1
        elif (str(reaction.emoji) == TOXIC_EMOJI or (reaction.custom_emoji and reaction.emoji.name == TOXIC_EMOJI)):
            karma_dict[author][3] += 1

@client.event
async def on_reaction_remove(reaction, user):
    author = reaction.message.author
    if (author != user and user != client.user):
        if (author not in karma_dict):
            karma_dict[author] = [0,0,0,0]
        if (str(reaction.emoji) == AUT_EMOJI or (reaction.custom_emoji and reaction.emoji.name == AUT_EMOJI)):
            karma_dict[author][0] -= 1
        elif (str(reaction.emoji) == NORM_EMOJI or (reaction.custom_emoji and reaction.emoji.name == NORM_EMOJI)):
            karma_dict[author][1] -= 1
        elif (str(reaction.emoji) == NICE_EMOJI or (reaction.custom_emoji and reaction.emoji.name == NICE_EMOJI)):
            karma_dict[author][2] -= 1
        elif (str(reaction.emoji) == TOXIC_EMOJI or (reaction.custom_emoji and reaction.emoji.name == TOXIC_EMOJI)):
            karma_dict[author][3] -= 1


@client.command(name='check',
                description="See how autistic one person is",
                brief="check up on one person",
                aliases=[],
                pass_context=True)
async def check(context):
    for member in context.message.mentions:
        if (member not in karma_dict):
            karma_dict[member] = [0,0,0,0]
        response = member.display_name + " is "
        response += (str(get_autism_percent(member)) + "% autistic" if (get_autism_percent(member) >= get_normie_percent(member)) else str(get_normie_percent(member)) + "% normie")
        response += " and " + (str(get_toxc_percent(member)) + "% toxic." if (get_toxc_percent(member) >= get_nice_percent(member)) else str(get_nice_percent(member)) + "% nice.")
        await client.send_message(context.message.channel, response)

@client.command(name='remove',
                description="Remove user from the spectrum",
                brief="Remove user from the spectrum",
                aliases=[],
                pass_context=True)
async def remove(context):
    for member in context.message.mentions:
        if (member in karma_dict):
            karma_dict.pop(member)
            await client.send_message(context.message.channel, context.message.author.mention + " has been removed")

@client.command()
async def square(number):
    squared_value = int(number) * int(number)
    await client.say(str(number) + " squared is " + str(squared_value))

def is_me(m):
    return m.author == client.user

def get_autism_percent(m):
    if (karma_dict[m][0] + karma_dict[m][1] == 0):
        return 0
    return ((karma_dict[m][0] - karma_dict[m][1]) / (karma_dict[m][0] + karma_dict[m][1])) * 100
def get_normie_percent(m):
    if (karma_dict[m][0] + karma_dict[m][1] == 0):
        return 0
    return ((karma_dict[m][1] - karma_dict[m][0]) / (karma_dict[m][1] + karma_dict[m][0])) * 100
def get_nice_percent(m):
    if (karma_dict[m][2] + karma_dict[m][3] == 0):
        return 0
    return ((karma_dict[m][2] - karma_dict[m][3]) / (karma_dict[m][2] + karma_dict[m][3])) * 100
def get_toxc_percent(m):
    if (karma_dict[m][2] + karma_dict[m][3] == 0):
        return 0
    return ((karma_dict[m][3] - karma_dict[m][2]) / (karma_dict[m][3] + karma_dict[m][2])) * 100

@client.command(name='spectrum',
        description="Vote :pech: for toxic, 🅱️for autistic, ❤ for nice, and :reee: for normie.",
                brief="Check if autistic.",
                aliases=[],
                pass_context=True)
async def spectrum(context):
    x = []
    y = []
    names = []
    for member in karma_dict:
        x.append(get_nice_percent(member) / 10)
        y.append(get_autism_percent(member) / 10)
        names.append(member.display_name)
    spectrum_gen.generate(x, y, names)
    with open('res/foo.png', 'rb') as f:
        await client.send_file(context.message.channel, f, content="Here you go, " + context.message.author.mention)


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
    await client.change_presence(game=Game(name="With Server Perms"))
    try:
        karma_dict = load_karma()
    except:
        print("could not load previous reactions")
    print("Logged in as " + client.user.name)

def save_karma(karma):
    with open('data/karma.pkl', 'wb') as f:
        pickle.dump(karma, f, pickle.HIGHEST_PROTOCOL)

def load_karma():
    with open('data/karma.pkl', 'rb') as f:
        return pickle.load(f)


async def list_servers():
    await client.wait_until_ready()
    while not client.is_closed:
        print("Current servers:")
        for server in client.servers:
            print(server.name)
            try:
                save_karma(karma_dict)
            except:
                print("could not save reaction data")
        await asyncio.sleep(600)


client.loop.create_task(list_servers())
client.run(TOKEN)
