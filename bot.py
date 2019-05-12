#!/usr/bin/env python3
# -*- coding: utf-8 -*-

import random
import re
import asyncio, aiohttp
from collections import deque
import discord
import time
from pytz import timezone
from discord import Game, message
from discord.ext import commands
from discord.ext.commands import Bot
from datetime import datetime

from src.config import secret_token, session, default_cmds
from src.smart_message import smart_message
from src.smart_command import smart_command
from src.emoji_manager import emoji_manager, is_animated
from src.list_embed import list_embed, dank_embed
from src.server_settings import server_settings
from src.models import User, Admin, Command
from src.smart_player import smart_player

BOT_PREFIX = ("?", "!")
TOKEN = secret_token

JOHNYS_ID = '214037134477230080'
GHOSTS_ID = '471864040998699011'
MATTS_ID = '168722115447488512'

cache = {}
smart_commands = {}
karma_dict = {}
admins = {}
settings_dict = {}
emoji_managers = {}
tracked_messages = deque([], maxlen=30)
deletable_messages = []
starboarded_messages = []

players = {}
client = Bot(command_prefix=BOT_PREFIX)
client.remove_command('help')

@client.command(name='help',
                description='Lists each command with a brief description.',
                brief='Gives help',
                pass_context=True)
async def help(ctx):
    help_txt = '```Commands:\n'
    settings = settings_dict[ctx.message.channel.server]
    args = ctx.message.content.split(' ')

    if len(args) > 1:
        for name, command in default_cmds.items():
            if args[1] in command.get_aliases():
                await client.send_message(ctx.message.channel, command.format_help(BOT_PREFIX[1] + args[1], settings=settings))
                return

    for name, command in default_cmds.items():
        help_txt += '{0:15} {1}'.format(command.name, command.get_brief()) + '\n'
    help_txt += '\nType !help <command> for command specific help```'

    await client.send_message(ctx.message.channel, help_txt)

@client.command(pass_context=True)
async def pause(context):
    if not settings_dict[context.message.channel.server].music_enabled: return
    player = players[context.message.channel.server.id]
    player.pause()

@client.command(pass_context=True)
async def resume(context):
    if not settings_dict[context.message.channel.server].music_enabled: return
    player = players[context.message.channel.server.id]
    player.resume()

@client.event
async def on_server_emojis_update(before, after):
    try: server = before[0].server
    except: server = after[0].server
    if not settings_dict[server].manage_emojis: return

    if len(before) == len(after): # if renamed
        diff = [i for i in range(len(after)) if before[i].name != after[i].name]
        for i in diff:
            if is_animated(before[i]): continue
            await emoji_managers[server.id].rename_emoji(before[i], after[i])

    elif len(before) > len(after): # if removed
        for emoji in [emoji for emoji in before if emoji not in after]:
            if is_animated(emoji): continue
            emoji_managers[server.id].del_emoji(emoji)

    elif len(after) > len(before): # if added
        for emoji in [emoji for emoji in after if emoji not in before]:
            if is_animated(emoji): continue
            await emoji_managers[server.id].add_emoji(emoji)


@client.event
async def on_server_join(server):
    print("JOINED NEW SERVER: %s - %s (%s)" % (server.name, server.id, server.member_count))
    await on_ready()

@client.event
async def on_message_delete(message):
    settings = settings_dict[message.channel.server]
    if message.channel.name == 'bangers': return  #TODO
    if message.author != client.user and message.id not in deletable_messages and settings.repost_del_msg:
        est = get_datetime(message.timestamp)
        em = discord.Embed(title=est.strftime("%Y-%m-%d %I:%M %p"), description=message.content, colour=0xff002a)
        em.set_author(name=message.author.display_name, icon_url=message.author.avatar_url)
        if message.embeds:
            em.set_image(url=message.embeds[0]['url'])
        elif message.attachments:
            em.set_image(url=message.attachments[0]['url'])
        del_msg = await client.send_message(message.channel, embed=em)
        for sm in tracked_messages:
            if (sm.peek().id == message.id):
                sm.embed = del_msg
    elif message.id in deletable_messages:
        deletable_messages.remove(message.id)

@client.event
async def on_message_edit(before, after):
    server = before.channel.server
    if before.author == client.user:
        return
    print('<%s>[%s](%s) - [%s](%s) %s(%s): %s CHANGED TO:' % (datetime.now(), server.name, server.id, before.channel.name, before.channel.id, before.author.display_name, before.author.id, before.content))
    print('<%s>[%s](%s) - [%s](%s) %s(%s): %s' % (datetime.now(), server.name, server.id, after.channel.name, after.channel.id, after.author.display_name, after.author.id, after.content))
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
    server = reaction.message.channel.server
    settings = settings_dict[server]
    print (reaction.emoji)
    for e in reaction.message.embeds:
        author_name, author_avatar = '',''
        try:
            author_name = e['author']['name']
            author_avatar = e['author']['icon_url']
        except: pass # not the type of embed we were expecting
        # this won't work if the user has default avatar
        real_author = discord.utils.get(server.members, name=author_name, avatar_url=author_avatar)
        if (real_author != None):
            author = real_author

    if settings.edit_emoji in str(reaction.emoji):
        await add_popup(reaction.message)

    if settings.starboard_emoji in str(reaction.emoji):
        if reaction.count == settings.starboard_threshold:
            await starboard_post(reaction.message, server)

    if ((author != user or user.id == JOHNYS_ID or user.id == MATTS_ID) and author != client.user and user != client.user):
        if (author.id not in karma_dict):
            karma_dict[author.id] = [2,2,2,2,0]
            new_user = User(author.id, karma_dict[author.id])
            session.add(new_user)

        if settings.aut_emoji in str(reaction.emoji):
            karma_dict[author.id][0] += 1
        elif settings.norm_emoji in str(reaction.emoji):
            karma_dict[author.id][1] += 1
        elif settings.nice_emoji in str(reaction.emoji):
            karma_dict[author.id][2] += 1
        elif settings.toxic_emoji in str(reaction.emoji):
            karma_dict[author.id][3] += 1
        elif settings.bot_emoji in str(reaction.emoji):
            karma_dict[author.id][4] += 1
        update_user(author.id)

@client.event
async def on_reaction_remove(reaction, user):
    author = reaction.message.author
    server = reaction.message.channel.server
    settings = settings_dict[server]
    for e in reaction.message.embeds:
        try:
            author_name = e['author']['name']
            author_avatar = e['author']['icon_url']
        except:
            author_avatar = ''
        real_author = discord.utils.get(server.members, name=author_name, avatar_url=author_avatar)
        if (real_author != None):
            author = real_author

    if settings.edit_emoji in str(reaction.emoji):
        for react in reaction.message.reactions:
            if settings.edit_emoji in str(react): return
        await delete_popup(reaction.message)

    if ((author != user or user.id == JOHNYS_ID) and author != client.user):
        if (author.id not in karma_dict):
            karma_dict[author.id] = [2,2,2,2,0]
            new_user = User(author.id, karma_dict[author.id])
            session.add(new_user)
        if settings.aut_emoji in str(reaction.emoji):
            karma_dict[author.id][0] -= 1
        elif settings.norm_emoji in str(reaction.emoji):
            karma_dict[author.id][1] -= 1
        elif settings.nice_emoji in str(reaction.emoji):
            karma_dict[author.id][2] -= 1
        elif settings.toxic_emoji in str(reaction.emoji):
            karma_dict[author.id][3] -= 1
        elif settings.bot_emoji in str(reaction.emoji):
            karma_dict[author.id][4] -= 1

        update_user(author.id)


@client.command(pass_context=True)
async def whois(ctx):
    usr = await client.get_user_info(ctx.message.clean_content.split()[1])
    await client.send_message(ctx.message.channel, usr.name + '#' + usr.discriminator)

@client.command(pass_context=True)
async def roleids(ctx):
    channel = ctx.message.channel
    lem = list_embed(channel.server.name, '')
    lem.name = "Role IDs"
    for role in channel.server.roles:
        if not role.is_everyone:
            lem.add(role.name, role.id)
    print(lem.get_embed().to_dict())
    await client.send_message(channel, embed=lem.get_embed())

@client.command(pass_context=True)
async def test(context):
    author = context.message.author
    channel = context.message.channel
    server = context.message.channel.server
    if (author.id != JOHNYS_ID and author.id != GHOSTS_ID):
        await client.send_message(context.message.channel, "it works")
        return

    await client.send_message(context.message.channel, embed=lem.get_embed())

def get_datetime(timestamp):
    utc = timestamp.replace(tzinfo=timezone('UTC'))
    est = utc.astimezone(timezone('US/Eastern'))
    return est

@client.event
async def on_member_join(member):
    print("%s joined server: %s" % (member.name, member.server.name))
    try:
        default_role = discord.utils.get(member.server.roles, id=settings_dict[member.server].default_role_id)
        await client.add_roles(member, default_role)
    except:
        print("could not add %s to %s" % (member.display_name, 'default role'))

def log_message(msg):
    url = ''
    try:
        url = message.embeds[0]['url'] if message.embeds else ''
        url = message.attachments[0]['url'] if message.attachments else ''
    except: pass
    log = str(datetime.now())
    try: log += '[%s](%s) - ' % (msg.channel.server.name, msg.channel.server.id)
    except: log += '[err](err) - '
    try: log += '[%s](%s) ' % (msg.channel.name, msg.channel.id)
    except: log += '[err](err) '
    try: log += '%s(%s): ' % (msg.author.display_name, msg.author.id)
    except: log += 'err(err): '
    try: log += '%s <%s>' % (msg.clean_content, url)
    except: log += 'err <err>'
    return log

@client.event
async def on_message(message):
    # forward dms
    if message.channel.is_private and message.author != client.user:
        await client.send_message(await client.get_user_info(JOHNYS_ID), message.author.mention + ': ' + message.content)
        return

    server = message.channel.server
    cache[server]['messages'][message.channel] = None
    print(log_message(message))
    settings = settings_dict[server]

    if not message.author.bot:

        # check for built in commands
        args = message.clean_content.split(' ')
        if args and args[0] and  args[0][0] in BOT_PREFIX:
            for name, command in default_cmds.items():
                if args[0][1:] in command.get_aliases():
                    if not await command.execute(message, client, players=players, settings=settings, karma_dict=karma_dict,
                            session=session, cache=cache, smart_commands=smart_commands, admins=admins, emoji_managers=emoji_managers):
                        await client.send_message(message.channel, command.format_help(args[0], settings=settings))

        # check for commands in this file
        await client.process_commands(message)

        # bump/insert emojis if necessary
        if settings.manage_emojis: await emoji_managers[server.id].scan(message)

        # check for user commands
        for command in smart_commands[int(message.channel.server.id)]:
            if (command.triggered(message.content)):
                resp = command.generate_response(message.author, message.content)
                update_command(command.raw_trigger, command.raw_response, command.count, command.server, command.author_id)
                reacts = command.generate_reacts()
                if resp:
                    await client.send_message(message.channel, resp)
                for react in reacts:
                    await client.add_reaction(message, react)
                break

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

async def starboard_post(message, server):
    starboard_ch = discord.utils.get(server.channels, name='starboard', type=ChannelType.text)
    if message.id in starboarded_messages or not starboard_ch or message.author == client.user:
        return
    starboarded_messages.append(message.id)
    est = get_datetime(message.timestamp)
    em = discord.Embed(title=est.strftime("%Y-%m-%d %I:%M %p"), description=message.content, colour=0x42f468)
    em.set_author(name=message.author.display_name, icon_url=message.author.avatar_url)
    if message.embeds:
        em.set_image(url=message.embeds[0]['url'])
    elif message.attachments:
        em.set_image(url=message.attachments[0]['url'])
    await client.send_message(starboard_ch, embed=em)

@client.event
async def on_ready():
    initialize_settings()
    initialize_players()
    initialize_scores()
    initialize_admins()
    initialize_commands()
    initialize_cache()
    await initialize_emoji_managers()
    print("Logged in as " + client.user.name)
    await client.change_presence(game=Game(name="the tragedy of darth plagueis the wise", url='https://www.twitchquotes.com/copypastas/2202', type=2))

async def list_servers():
    await client.wait_until_ready()
    while not client.is_closed:
        print("Current servers:")
        for server in client.servers:
            print("%s - %s (%s)" % (server.name, server.id, server.member_count))
        await asyncio.sleep(600)

async def initialize_emoji_managers():
    from src.emoji_manager import emoji_manager
    for server in client.servers:
        emoji_managers[server.id] = emoji_manager(client, server, deletable_messages)
        if settings_dict[server].manage_emojis:
            await emoji_managers[server.id].clean()

def initialize_players():
    for server in client.servers:
        players[server.id] = smart_player(client)

def initialize_settings():
    for server in client.servers:
        settings_dict[server] = server_settings(session, server)

def initialize_admins():
    for server in client.servers:
        settings = server_settings(session, server)
        admins[int(server.id)] = [int(admin) for admin in settings.admins_ids]

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

def initialize_cache():
    for server in client.servers:
        cache[server] = {
            'messages' : {}
        }

def initialize_commands():
    command_list = session.query(Command).all()
    for server in client.servers:
        smart_commands.setdefault(int(server.id), [])
    for command in command_list:
        smart_commands.setdefault(command.server_id, [])
        smart_commands[command.server_id].append(smart_command(command.trigger.replace(str(command.server_id), '', 1), command.response, command.count, client.get_server(str(command.server_id)), command.author_id))
    for server, cmds in smart_commands.items():
        smart_commands[server].sort()

def update_user(disc_id, delete=False):
    if (delete):
        session.query(User).filter_by(discord_id = disc_id).delete()
        session.commit()
        return
    new_data = {
            'aut_score': karma_dict[disc_id][0],
            'norm_score': karma_dict[disc_id][1],
            'nice_score': karma_dict[disc_id][2],
            'toxic_score': karma_dict[disc_id][3],
            'awareness_score': karma_dict[disc_id][4]
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
    session.query(Admin).filter_by(discord_id = int(member.id)).update(new_data)
    session.commit()

def update_command(triggerkey, response, count, server, author_id, delete=False):
    if (delete):
        session.query(Command).filter_by(trigger = server.id + triggerkey).delete()
        session.commit()
        return
    new_data = {
            'server_id': server.id,
            'response': response,
            'count': count,
            'author_id': int(author_id)
            }
    session.query(Command).filter_by(trigger = server.id + triggerkey).update(new_data)
    session.commit()

client.loop.create_task(list_servers())
client.run(TOKEN)
