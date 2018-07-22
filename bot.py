#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import random
import re
import asyncio
import aiohttp
from collections import deque
import discord
import time
from pytz import timezone
import pytz
from discord import Game
from discord import message
from discord.ext.commands import Bot

from src.config import secret_token, session
from src.smart_message import smart_message
from src.list_embed import list_embed
from src.models import User
from src.smart_player import smart_player
import src.spectrum_gen as spectrum_gen

BOT_PREFIX = ("?", "!")
TOKEN = secret_token

PECHS_ID = '178700066091958273'
JOHNYS_ID = '214037134477230080'
MATTS_ID = '168722115447488512'

karma_dict = {}
tracked_messages = deque([], maxlen=20)

AUT_EMOJI = "🅱"
NORM_EMOJI = "reee"
NICE_EMOJI = "❤"
TOXIC_EMOJI = "pech"
EDIT_EMOJI = "📝"


client = Bot(command_prefix=BOT_PREFIX)
player = smart_player()


@client.command(name='join',
                description="Joins the caller's voice channel",
                brief="joins voice.",
                pass_context=True)
async def join(context):
    if not (player.is_connected()):
        voice = await client.join_voice_channel(context.message.author.voice.voice_channel)
        player.voice = voice
    else:
        player.voice.move_to(context.message.author.voice.voice_channel)

@client.command(name='skip',
                description="Skip current song",
                brief="skip song",
                pass_context=True)
async def skip(context):
    player.skip()
    asyncio.sleep(2)
    await client.send_message(context.message.channel, "Now playing: " + player.name)

@client.command(name='pause',
                description="Pauses current song",
                brief="pause song",
                aliases=['stop'],
                pass_context=True)
async def pause(context):
    player.pause()

@client.command(name='resume',
                description="Resume current song",
                brief="resume song",
                pass_context=True)
async def resume(context):
    player.resume()

@client.command(name='play',
                description="!play [encounter|boss|exploration|town]",
                brief="play some dnd music",
                pass_context=True)
async def play(context):
    if not discord.opus.is_loaded():
        discord.opus.load_opus('res/libopus.so')
    if not (player.is_connected()):
        voice = await client.join_voice_channel(context.message.author.voice.voice_channel)
        player.voice = voice
    else:
        player.voice.move_to(context.message.author.voice.voice_channel)

    arg = context.message.content.split(' ')
    if (len(arg) == 2):
        if ('playlist' in arg[1]):
            await client.send_message(context.message.channel, "Shuffling playlist...")
        elif ('track' in arg[1]):
            await client.send_message(context.message.channel, "Playing Song...")
        elif ('youtu' in arg[1]):
            await client.send_message(context.message.channel, "Playing youtube...")
        else:
            player.play(arg[1])
            await client.send_message(context.message.channel, "Now playing: " + player.name)
    else:
        await client.send_message(context.message.channel, "Play what, " + context.message.author.mention + "?")

    #await client.play_audio(f)

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
        'Yep.',
        'Possibly.'
    ]
    await client.say(random.choice(possible_responses) + ", " + context.message.author.mention)

@client.event
async def on_message_delete(message):
    # if (message.author.id == PECHS_ID):
    if not is_me(message):
        est = get_datetime(message.timestamp)
        em = discord.Embed(title=est.strftime("%Y-%m-%d %I:%M %p"), description=message.content, colour=0xff002a)
        em.set_author(name=message.author.display_name, icon_url=message.author.avatar_url)
        del_msg = await client.send_message(message.channel, embed=em)
        for sm in tracked_messages:
            if (sm.peek().id == message.id):
                sm.embed = del_msg

@client.event
async def on_message_edit(before, after):
    for sm in tracked_messages:
        if (sm.add_edit(before, after)):
            await edit_popup(before)
            return
    sm = smart_message(before)
    sm.add_edit(before, after)
    tracked_messages.append(sm)
    
def update_user(disc_id):
    new_data = {
            'aut_score': karma_dict[disc_id][0],
            'norm_score': karma_dict[disc_id][1],
            'nice_score': karma_dict[disc_id][2],
            'toxic_score': karma_dict[disc_id][3]
            }
    session.query(User).filter_by(discord_id = disc_id).update(new_data)
    session.commit()

@client.event
async def on_reaction_add(reaction, user):
    author = reaction.message.author
    for e in reaction.message.embeds:
        author_name, author_avatar = '',''
        try:
            author_name = e['author']['name']
            author_avatar = e['author']['icon_url']
        except:
            pass # not the type of embed we were expecting
        real_author = find_member(author_name, author_avatar, reaction.message.channel.server)
        if (real_author != None):
            author = real_author

    if ((author != user or user.id == JOHNYS_ID or user.id == MATTS_ID) and author != client.user):
        if (author.id not in karma_dict):
            karma_dict[author.id] = [2,2,2,2]
            new_user = User(author.id, karma_dict[author.id])
            session.add(new_user)

        if (str(reaction.emoji) == AUT_EMOJI or (reaction.custom_emoji and reaction.emoji.name == AUT_EMOJI)):
            karma_dict[author.id][0] += 1
        elif (str(reaction.emoji) == NORM_EMOJI or (reaction.custom_emoji and reaction.emoji.name == NORM_EMOJI)):
            karma_dict[author.id][1] += 1
        elif (str(reaction.emoji) == NICE_EMOJI or (reaction.custom_emoji and reaction.emoji.name == NICE_EMOJI)):
            karma_dict[author.id][2] += 1
        elif (str(reaction.emoji) == TOXIC_EMOJI or (reaction.custom_emoji and reaction.emoji.name == TOXIC_EMOJI)):
            karma_dict[author.id][3] += 1
        elif (str(reaction.emoji) == EDIT_EMOJI or (reaction.custom_emoji and reaction.emoji.name == EDIT_EMOJI)):
            await add_popup(reaction.message)
        update_user(author.id)

@client.event
async def on_reaction_remove(reaction, user):
    author = reaction.message.author
    for e in reaction.message.embeds:
        try:
            author_name = e['author']['name']
            author_avatar = e['author']['icon_url']
        except:
            pass
        real_author = find_member(author_name, author_avatar, reaction.message.channel.server)
        if (real_author != None):
            author = real_author
    if ((author != user or user.id == JOHNYS_ID) and author != client.user):
        if (author.id not in karma_dict):
            karma_dict[author.id] = [2,2,2,2]
            new_user = User(author.id, karma_dict[author.id])
            session.add(new_user)
        if (str(reaction.emoji) == AUT_EMOJI or (reaction.custom_emoji and reaction.emoji.name == AUT_EMOJI)):
            karma_dict[author.id][0] -= 1
        elif (str(reaction.emoji) == NORM_EMOJI or (reaction.custom_emoji and reaction.emoji.name == NORM_EMOJI)):
            karma_dict[author.id][1] -= 1
        elif (str(reaction.emoji) == NICE_EMOJI or (reaction.custom_emoji and reaction.emoji.name == NICE_EMOJI)):
            karma_dict[author.id][2] -= 1
        elif (str(reaction.emoji) == TOXIC_EMOJI or (reaction.custom_emoji and reaction.emoji.name == TOXIC_EMOJI)):
            karma_dict[author.id][3] -= 1
        elif (str(reaction.emoji) == EDIT_EMOJI or (reaction.custom_emoji and reaction.emoji.name == EDIT_EMOJI)):
            for react in reaction.message.reactions:
                if (str(reaction.emoji) == EDIT_EMOJI or (reaction.custom_emoji and reaction.emoji.name == EDIT_EMOJI)):
                    return
            await delete_popup(reaction.message)

        update_user(author.id)


@client.command(name='check',
                description="See how autistic one person is",
                brief="check up on one person",
                aliases=[],
                pass_context=True)
async def check(context):
    for member in context.message.mentions:
        if (member == client.user):
            await client.send_message(context.message.channel, "Leave me out of this, " + context.message.author.mention)
            return
        if (member.id not in karma_dict):
            karma_dict[member.id] = [2,2,2,2]
            new_user = User(member.id, karma_dict[member.id])
            session.add(new_user)
        response = member.display_name + " is "
        response += ("{:3.1f}% autistic".format(get_autism_percent(member.id)) if (get_autism_percent(member.id) >= get_normie_percent(member.id)) else "{:3.1f}% normie".format(get_normie_percent(member.id)))
        response += " and " + ("{:3.1f}% toxic.".format(get_toxc_percent(member.id)) if (get_toxc_percent(member.id) >= get_nice_percent(member.id)) else "{:3.1f}% nice.".format(get_nice_percent(member.id)))
        await client.send_message(context.message.channel, response)

@client.command(pass_context=True)
async def test(context):
    x = [1, -3, 5, 7, -8, 3, -5, -7]
    y = [-1, 2, -7, 5, 1, 0, 4, -6]
    names = ['p🅱ch', 'johny', 'test', 'raines', 'hello', 'hi', 'owo', 'I hate sand']
    spectrum_gen.generate(x, y, names)
    with open('res/foo.png', 'rb') as f:
        await client.send_file(context.message.channel, f, content="Here you go, " + context.message.author.mention)


@client.command(name='remove',
        description="Remove user from the spectrum",
        brief="Remove user from the spectrum",
        aliases=[],
        pass_context=True)
async def remove(context):
    for member in context.message.mentions:
        if (member.id in karma_dict):
            karma_dict.pop(member.id)
            await client.send_message(context.message.channel, member.mention + " has been removed")

@client.command()
async def square(number):
    squared_value = int(number) * int(number)
    await client.say(str(number) + " squared is " + str(squared_value))

def find_member(name, icon, server):
    for m in server.members:
        if (name == m.display_name and icon == m.avatar_url):
            return m
    return None

def find_by_id(mem_id, server):
    for m in server.members:
        if (mem_id == m.id):
            return m
    return None

def get_datetime(timestamp):
    utc = timestamp.replace(tzinfo=timezone('UTC'))
    est = utc.astimezone(timezone('US/Eastern'))
    return est

def is_me(m):
    return m.author == client.user

def get_autism_percent(m):
    if (karma_dict[m][0] + karma_dict[m][1] == 0):
        return 
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
    client.send_typing(context.message.channel)
    x = []
    y = []
    names = []
    for mem_id in karma_dict:
        toxic = get_toxc_percent(mem_id)
        nice = get_nice_percent(mem_id)
        aut = get_autism_percent(mem_id)
        norm = get_normie_percent(mem_id)
        if (toxic > nice):
            x.append(-1*(toxic) / 10)
        else:
            x.append(nice / 10)
        if (norm > aut):
            y.append(-1*(norm) / 10)
        else:
            y.append(aut / 10)
        #y.append((get_autism_percent(member) - get_normie_percent(member)) / 10)
        member = find_by_id(mem_id, context.message.channel.server)
        if (member is not None) :
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

@client.command(pass_context=True)
async def log(context):
    msgs = []
    do_filter = bool(context.message.mentions)
    try:
        num = int(re.search(r'\d+', context.message.content).group())
    except:
        num = 0
    num = 25 if num == 0 or num > 25 else num
    author = client.user

    em = discord.Embed(title='Last %s messages' % (str(num)), description='My Embed Content.', colour=0x5998ff)
    if (do_filter):
        author = context.message.mentions[0]
    lembed = list_embed('Last %s messages' % (str(num)), 'here you go', author)
    async for message in client.logs_from(context.message.channel, limit=1000):
        if (not do_filter or message.author == context.message.mentions[0]):
            msgs.append(message)
        if (len(msgs) >= num):
            break
    for message in reversed(msgs):
        lembed.add(author.display_name, message.content)
    await client.send_message(context.message.channel, embed=lembed.get_embed())

@client.event
async def on_ready():
    await client.change_presence(game=Game(name="not spotify apparently"))
    print("Logged in as " + client.user.name)
    users = session.query(User).all()
    for user in users:
        karma_dict[user.discord_id] = user.as_entry()

async def edit_popup(message):
    for sm in tracked_messages:
        if (message.id == sm.peek().id or (sm.embed != None and message.id == sm.embed.id)):
            if (not sm.has_popup()):
                return
            else:
                lem = sm.add_popup()
                await client.edit_message(sm.popup, embed=lem)
async def add_popup(message):
    for sm in tracked_messages:
        if (message.id == sm.peek().id or (sm.embed != None and message.id == sm.embed.id)):
            if (not sm.has_popup()):
                lem = sm.add_popup()
                popup = await client.send_message(message.channel, embed=lem)
                sm.popup = popup
            else:
                await edit_popup(message)

async def delete_popup(message):
    for sm in tracked_messages:
        if (message.id == sm.peek().id):
            if (sm.has_popup()):
                await client.delete_message(sm.popup)
                sm.popup = None



async def list_servers():
    await client.wait_until_ready()
    while not client.is_closed:
        print("Current servers:")
        for server in client.servers:
            print(server.name)
        await asyncio.sleep(600)


client.loop.create_task(list_servers())
client.run(TOKEN)
