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
from discord.ext import commands
from discord.ext.commands import Bot

from src.config import secret_token, session
from src.smart_message import smart_message
from src.list_embed import list_embed
from src.models import User, Admin
from src.smart_player import smart_player
import src.spectrum_gen as spectrum_gen

BOT_PREFIX = ("?", "!")
TOKEN = secret_token

PECHS_ID = '178700066091958273'
JOHNYS_ID = '214037134477230080'
MATTS_ID = '168722115447488512'
SIMONS_ID = '103027947786473472'
MONKEYS_ID = '189528269547110400'


ROLES_DICT = {
    "black santa" : "🎅🏿",
    "whale" : "🐋",
    "fox" : "🦊",
    "pink" : "pink",
    "back on top soon" : "🔙🔛🔝🔜",
    "nsfw" : "nsfw",
    "pugger" : "pugger"
}

DEFAULT_ROLE = 'Admin'

karma_dict = {}
admins = {}
tracked_messages = deque([], maxlen=20)

AUT_EMOJI = "🅱"
NORM_EMOJI = "reee"
NICE_EMOJI = "❤"
TOXIC_EMOJI = "pech"
EDIT_EMOJI = "📝"

players = {}
client = Bot(command_prefix=BOT_PREFIX)
#client.remove_command('help')

@client.command(name='skip',
                description="Skip current song",
                brief="skip song",
                pass_context=True)
@commands.cooldown(1, 1, commands.BucketType.server)
async def skip(context):
    player = players[context.message.channel.server.id]
    name = await player.skip()
    if (name):
        await client.send_message(context.message.channel, "Now playing: " + name)
    else:
        await client.send_message(context.message.channel, "No songs left. nice job. bye.")
        if (player.is_connected()):
            await player.voice.disconnect()

@client.command(name='pause',
                description="Pauses current song",
                brief="pause song",
                aliases=['stop'],
                pass_context=True)
async def pause(context):
    player = players[context.message.channel.server.id]
    player.pause()

@client.command(name='clear',
                description="Remove all songs from queue.",
                brief="clear queue",
                pass_context=True)
async def clear(context):
    player = players[context.message.channel.server.id]
    await client.send_message(context.message.channel, "Removed %d songs from queue." % len(player.q))
    player.clearq()

@client.command(name='resume',
                description="Resume current song",
                brief="resume song",
                pass_context=True)
async def resume(context):
    player = players[context.message.channel.server.id]
    player.resume()
@client.command(name='adde',
                description="Add a song to queue",
                brief="Add song",
                pass_context=True)
@commands.cooldown(1, 5, commands.BucketType.server)
async def adde(context):
    #await client.send_typing(context.message.channel)
    await client.send_message(context.message.channel, '')


@client.command(name='play',
                description="![play|add] [url|search]",
                brief="play tunes",
                aliases=['add'],
                pass_context=True)
@commands.cooldown(1, 2, commands.BucketType.server)
async def play(context):
    player = players[context.message.channel.server.id]
    await client.send_typing(context.message.channel)
    if not discord.opus.is_loaded():
        discord.opus.load_opus('res/libopus.so')
    if not (player.is_connected()):
        voice = await client.join_voice_channel(context.message.author.voice.voice_channel)
        player.voice = voice
    else:
        player.voice.move_to(context.message.author.voice.voice_channel)

    arg = context.message.content.split(' ')
    add = arg[0] == '!add'
    message = ''
    if (len(arg) > 1):
        if ('/playlist/' in arg[1]):
            urls = await player.add_spotify_playlist(arg[1])
            message = "Queuing \"" + urls[0] + "\"."
            del urls[0]
            await player.add_url(urls[0])
            name = await player.play()
            for track in urls:
                await player.add_url(track)
            if (name):
                message += "\nPlaying: " + name
        elif ('/track/' in arg[1]):
            if (add):
                name = await player.add_url(arg[1]);
                if (name):
                    message = 'Added: ' + name
            else:
                await player.add_url_now(arg[1]);
                name = await player.play()
                if (name):
                    message = "Now playing: " + name
        elif ('youtu' in arg[1]):
            if (add):
                await player.add_url(arg[1])
                message = 'Added'
            else:
                await player.add_url_now(arg[1])
                name = await player.play()
                if (name):
                    message = "Playing " + name
        elif ('town' in arg[1] or 'encounter' in arg[1] or 'boss' in arg[1] or 'exploration' in arg[1]):
            message = "Please pass in the url of the playlist."
        else:
            del arg[0]
            url = await player.get_youtube_url(' '.join(arg))
            if (add):
                await player.add_url(url)
                message = "Added: " + url
            else:
                await player.add_url_now(url)
                name = await player.play()
                if (name):
                    message = "Now Playing: " + url
    else:
        if (len(player.q) == 0):
            message = "Play what, " + context.message.author.mention + "?"
        else:
            name = await player.play()
            if (name):
                message = "Now playing: " + name

    await client.send_message(context.message.channel, message)

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
async def on_server_join(server):
    players[server.id] = smart_player()
    admins[server.id] = [server.owner.id]

@client.event
async def on_message_delete(message):
    # if (message.author.id == PECHS_ID):
    if not is_me(message):# and message.author.id not in ADMINS:
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
    await client.send_typing(context.message.channel)
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
    emojis = client.get_all_emojis()
    for emoji in emojis:
        if (emoji.name == 'reee'):
            NORM_EMOJI_OBJ = str(emoji)
        elif (emoji.name == 'pech'):
            TOXIC_EMOJI_OBJ = str(emoji)

    await client.send_message(context.message.channel, context.message.channel.id)
    await client.send_message(context.message.channel, next(client.get_all_emojis()))
    await client.send_message(context.message.channel, NORM_EMOJI_OBJ)


@client.command(name='remove',
        description="Remove users from the spectrum if they are a sad boi",
        brief="Remove user from the spectrum",
        aliases=[],
        pass_context=True)
async def remove(context):
    server = context.message.channel.server
    for member in context.message.mentions:
        if (member.id in karma_dict):
            karma_dict.pop(member.id)
            update_admin(member, server, delete=True)
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
                description="Vote :pech: for toxic, 🅱️for autistic, ❤ for nice, and :reee: for normie." ,
                brief="Check if autistic.",
                aliases=[],
                pass_context=True)
@commands.cooldown(1, 5, commands.BucketType.server)
async def spectrum(context):
    await client.send_typing(context.message.channel)
    x = []
    y = []
    names = []
    for mem_id in karma_dict:
        member = context.message.channel.server.get_member(mem_id)
        if (member is not None) :
            names.append(member.display_name)
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
    spectrum_gen.generate(x, y, names)
    with open('res/foo.png', 'rb') as f:
        await client.send_file(context.message.channel, f, content="Here you go, " + context.message.author.mention)


@client.command(name='purge',
                description="Deletes the bot's spam.",
                brief="Delete spam.",
                pass_context=True)
@commands.cooldown(1, 5, commands.BucketType.server)
async def purge(context):
    channel = context.message.channel
    await client.send_typing(channel)
    if (int(context.message.author.id) in admins[int(channel.server.id)]):
        deleted = await client.purge_from(context.message.channel, limit=100, check=is_me)
        await client.send_message(channel, 'Deleted {} message(s)'.format(len(deleted)))
    else:
        await client.send_message(channel, 'lul %s' % context.message.author.mention)

@client.event
async def on_member_join(member):
    try:
        await client.add_roles(member, next(filter(lambda role: role.name == DEFAULT_ROLE, member.server.role_hierarchy)))
    except:
        print("could not add %s to %s" % (member.display_name, DEFAULT_ROLE))

@client.command(name='role',
                description="Assign yourself a role.",
                brief="Assign a role.",
                aliases=['join'],
                pass_context=True)
async def role(context):
    await client.send_typing(context.message.channel)
    arg = context.message.content.split(' ')
    member = context.message.author
    if (len(arg) < 2):
        requested_role = 'list'
    else:
        del arg[0]
        requested_role = ' '.join(arg)

    if (requested_role == 'list'):
        lembed = list_embed('Available Roles', '`!role [role]`', client.user)
        roles = "Available roles:\n"
        for roletag, rolename in ROLES_DICT.items():
            lembed.add(rolename, roletag)
        await client.send_message(context.message.channel, embed=lembed.get_embed())
    elif (requested_role.lower() in (name.lower() for name in ROLES_DICT)):
        filtered = filter(lambda role: role.name == ROLES_DICT[requested_role], member.server.role_hierarchy)
        action = 'Added'
        prep = 'to'
        try:
            role = next(filtered)
            if (role in member.roles):
                await client.remove_roles(member, role)
                action = 'Removed'
                prep = 'from'
            else:
                await client.add_roles(member, role)
        except:
            await client.send_message(context.message.channel, "Could not add %s to %s." % (context.message.author.mention, requested_role))
        else:
            await client.send_message(context.message.channel, "%s %s %s %s." % (action, context.message.author.mention, prep, requested_role))
    else:
        await client.send_message(context.message.channel, "I don't know that role, %s" % context.message.author.mention)

@client.command(pass_context=True)
async def admin(context):
    server = context.message.channel.server
    if ("remove" in context.message.content and context.message.author.id == server.owner.id):
        for member in context.message.mentions:
            if (member == context.message.author):
                await client.send_message(context.message.channel, "🤔")
                return
            update_admin(member, server, delete=True)
            admins[int(server.id)].remove(int(member.id))
            await client.send_message(context.message.channel, "Removed %s." % member.display_name)
        return

    if ("list" in context.message.content):
        print("serverid: " +server.id);
        print(admins)
        names = ''
        for userid in admins[int(server.id)]:
            names += (server.get_member(str(userid))).display_name + ' '
        await client.send_message(context.message.channel, names)
        return
    if (context.message.author.id == server.owner.id):
        for member in context.message.mentions:
            admins.setdefault(int(server.id), [])
            if (member.id not in admins[int(server.id)]):
                new_admin = Admin(server.id, member.id, member.name)
                session.add(new_admin)
                admins[int(server.id)].append(int(member.id))
                update_admin(member, server)
                await client.send_message(context.message.channel, "Added %s." % member.display_name)
    else:
        await client.send_message(context.message.channel, "Nice try. You have been reported.")

@client.command(pass_context=True)
async def log(context):
    await client.send_typing(context.message.channel)
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

@client.event
async def on_ready():
    initialize_players()
    initialize_scores()
    initialize_admins()
    print("Logged in as " + client.user.name)
    await client.change_presence(game=Game(name="spotify again"))

async def list_servers():
    await client.wait_until_ready()
    while not client.is_closed:
        print("Current servers:")
        for server in client.servers:
            print(server.name)
        await asyncio.sleep(600)

def initialize_players():
    for server in client.servers:
        players[server.id] = smart_player(client)

def initialize_admins():
    admin_list = session.query(Admin).all()
    for server in client.servers:
        admins[int(server.id)] = [int(server.owner.id)]
        admins[int(server.id)].append(int(JOHNYS_ID))
    for admin in admin_list:
        admins.setdefault(admin.server_id, [])
        admins[admin.server_id].append(admin.discord_id)

def initialize_roles():
    role_list = session.query(Role).all()
    for server in client.servers:
        roles.setdefault(int(server.id), [])
    for role in role_list:
        roles.setdefault(role.server_id, [])
        roles[role.server_id].append((role.target_role_id, role.required_role_id))

def initialize_scores():
    users = session.query(User).all()
    for user in users:
        karma_dict[user.discord_id] = user.as_entry()

def update_role(target_role_id, server_id, required_role_id=None, delete=False):
    if (delete):
        session.query(Role).filter_by(target_role_id = target_role_id).delete()
        session.commit()
        return
    new_data = {
            'server_id': server_id,
            'required_role_id': required_role_id
            }
def update_user(disc_id, delete=False):
    if (delete):
        session.query(User).filter_by(discord_id = disc_id).delete()
        session.commit()
        return
    new_data = {
            'aut_score': karma_dict[disc_id][0],
            'norm_score': karma_dict[disc_id][1],
            'nice_score': karma_dict[disc_id][2],
            'toxic_score': karma_dict[disc_id][3]
            }
    session.query(User).filter_by(discord_id = disc_id).update(new_data)
    session.commit()

def update_admin(member, server, delete=False):
    if (delete):
        session.query(Admin).filter_by(discord_id = member.id).delete()
        session.commit()
        return
    new_data = {
            'server_id': server.id,
            'username': member.name
            }
    session.query(Admin).filter_by(discord_id = member.id).update(new_data)
    session.commit()

client.loop.create_task(list_servers())
client.run(TOKEN)
