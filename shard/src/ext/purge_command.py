import discord
from discord.ext import commands
from discord.utils import snowflake_time

import re
from datetime import datetime, timedelta

TIME_REGEX = re.compile(r"(\d+)([m|s])")

# How far into the message history to search for messages to delete
MESSAGE_LIMIT = 10000


@commands.group()
async def purge(ctx):
    '''
    Purge a channel of a user's messages

    Usage: {prefix}purge id {message id} [true]
    or: {prefix}purge time {XXm|XXs}
    The first will purge all messages sent after the message corresponding to the
    message id of the user who sent that message. If true is passed, then all messages
    from now until that message will be deleted.

    Only searches the past 10000 messages sent in a channel for which to delete.
    If more than 1000 messages have been sent between the given message's id and
    when this command is run, then all of that user's messages may not get
    deleted.

    The second usage will delete all messages in the channel in the past number of
    minutes or seconds.

    NOTE: If the original message is deleted before the bot deletes it, all messages
    from the target message's author in the channel will be deleted depending on the
    timing of when the original message was deleted.
    '''
    if ctx.invoked_subcommand is None:
        await ctx.send("Need to use command with id or time parameter")
        return


@purge.command()
async def id(ctx, mid, inclusive=False):
    """
    Purge a channel of messages until a specific message is reached

    Usage: {prefix}purge id {message id} [true]
    Purges all messages sent after the message corresponding to the
    message id of the user who sent that message. If the optional last
    parameter is set to true or True then deleted messages will not be
    limited to just the one author but included all messages in the relevant
    time period. Any non "true" or "True" value passed in at the end will
    default to False.

    Only searches the past 10000 messages sent in a channel for which to delete.
    If more than 10000 messages have been sent between the given message's id and
    when this command is run, then all of that user's messages may not get
    deleted.

    NOTE: If the original message is deleted before the bot deletes it and after
    Architus has already started to go through the message history, all messages
    from the target message's author (up to 10,000) in the channel will be deleted.
    This race condition should have no effect on the command's behavior if
    all messages are going to be deleted.
    """
    message_id = None
    try:
        message_id = int(mid)
    except ValueError:
        await ctx.send("Message ID has an invalid format")
        return

    settings = ctx.bot.settings[ctx.guild]
    if ctx.author.id not in settings.admins_ids:
        await ctx.send("You do not have permissions to purge messaages")
        return

    try:
        original_message = await ctx.channel.fetch_message(message_id)
    except discord.NotFound:
        if not inclusive:
            await ctx.send("Message was already deleted and inclusive mod is not set")
            return
        sent_time = snowflake_time(message_id)
        original_message = None
    except discord.Forbidden:
        await ctx.send("Architus does not have permission to read message history")
        return
    except discord.HTTPException:
        await ctx.send("Failed to reach discord servers. Try again in a bit")
        return
    except Exception:
        await ctx.send("Something went wrong. :(")
        return

    messages_to_delete = []
    args = dict()
    if original_message is None:
        args['after'] = sent_time

    try:
        async for m in ctx.channel.history(limit=MESSAGE_LIMIT, **args):
            if inclusive or m.author == original_message.author:
                messages_to_delete.append(m)
            if original_message is not None and m.id == original_message.id:
                break
    except discord.Forbidden:
        await ctx.send("Architus does not have permission to view message history")
        return
    except discord.HTTPException:
        await ctx.send("Architus was unable to reach discord servers")
        return
    except Exception:
        await ctx.send("Something went wrong. :(")
        return

    try:
        async with ctx.channel.typing():
            for i in range(0, len(messages_to_delete), 100):
                await ctx.channel.delete_messages(messages_to_delete[i:i + 100])
            await ctx.send(f"Deleted {len(messages_to_delete)} message(s)")
    except discord.Forbidden:
        await ctx.send("Architus does not have permission to remove messages in this server")
    except discord.HTTPException:
        await ctx.send("Failed to reach discord servers. Try again in a bit")
    except Exception:
        await ctx.send("Something went wrong. :(")


@purge.command()
async def time(ctx, time_window: str, user: discord.Member = None):
    """
    Purge a channel of messages sent in the past X amount of time

    Usage: {prefix}purge time {XXm|XXs} [user]

    Deletes all messages in the channel in the past number of minutes or seconds. If
    a user is passed, such as by @ing them, then Architus will only delete messages
    by that user.

    NOTE: If the original message is deleted before the bot deletes it, all messages
    from the target message's author in the channel will be deleted depending on the
    timing of when the original message was deleted.
    """
    # Generally considered bad practice to use the utcnow datetime function.
    # However, the discord api specifically requires a timezone niave datetime
    # that represents the relevant time in the UTC timezone. This is exactly
    # what utcnow gives us so that's what we'll use.
    # See: https://discordpy.readthedocs.io/en/latest/api.html?highlight=channel#discord.TextChannel.history
    now = datetime.utcnow()
    time_param = TIME_REGEX.match(time_window.strip())
    if time_param is None:
        await ctx.send("Time value formatted improperly")
        return

    settings = ctx.bot.settings[ctx.guild]
    if ctx.author.id not in settings.admins_ids:
        await ctx.send("You do not have permissions to purge messaages")
        return

    if time_param[2] == 'm':
        mins = int(time_param[1])
        diff = timedelta(minutes=mins)
    elif time_param[2] == 's':
        secs = int(time_param[1])
        diff = timedelta(seconds=secs)

    earliest = now - diff

    try:
        if user == None:
            messages_to_delete = await ctx.channel.history(limit=MESSAGE_LIMIT, after=earliest).flatten()
        else:
            messages_to_delete = list()
            async for m in ctx.channel.history(limit=MESSAGE_LIMIT, after=earliest):
                if m.author == user:
                    messages_to_delete.append(m)
    except discord.Forbidden:
        await ctx.send("Architus does not have permission to read message history")
        return
    except discord.HTTPException:
        await ctx.send("Failed to reach discord servers. Try again in a bit")
        return
    except Exception:
        await ctx.send("Something went wrong. :(")
        return

    try:
        async with ctx.channel.typing():
            for i in range(0, len(messages_to_delete), 100):
                await ctx.channel.delete_messages(messages_to_delete[i:i + 100])
            await ctx.send(f"Deleted {len(messages_to_delete)} message(s)")
    except discord.Forbidden:
        await ctx.send("Architus does not have permission to remove messages "
                       "in this server.")
        return
    except discord.HTTPException:
        await ctx.send("Failed to reach discord servers. Try again in a bit")
        return
    except Exception:
        await ctx.send("Something went wrong. :(")
        return


def setup(bot):
    bot.add_command(purge)
