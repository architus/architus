import discord
from discord.ext import commands
from contextlib import suppress

from datetime import datetime, timedelta


@commands.group()
async def purge(ctx):
    '''
    Purge a channel of a user's messages

    Usage: {prefix}purge id {message id} [true]
    or: {prefix}purge time {XXm|XXs}
    The first will purge all messages sent after the message corresponding to the
    message id of the user who sent that message. If true is passed, then all messages
    from now until that message will be deleted.

    Only searches the past 500 messages sent in a channel for which to delete.
    If more than 500 messages have been sent between the given message's id and
    when this command is run, then all of that user's messages may not get
    deleted.

    The second usage will delete all messages in the channel in the past number of
    minutes or seconds.

    NOTE: If the original message is deleted before the bot deletes it, all messages
    from the target message's author in the channel will be deleted depending on the
    timing of when the original message was deleted.
    '''
    # Both subcommands will attempt to use the more efficient channel.delete_messages method
    # when possible. This operation is only valid when attempting to delete 100
    # or fewer messages. When this is not possible, the channel.purge method will
    # be used instead.
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

    Only searches the past 500 messages sent in a channel for which to delete.
    If more than 500 messages have been sent between the given message's id and
    when this command is run, then all of that user's messages may not get
    deleted.

    NOTE: If the original message is deleted before the bot deletes it, all messages
    from the target message's author in the channel will be deleted depending on the
    timing of when the original message was deleted.
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
        await ctx.send("Original message has already been deleted. "
                       "Try again with different message.")
        return
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
    async for m in ctx.channel.history(limit=500):
        if inclusive or m.author == original_message.author:
            messages_to_delete.append(m)
        if m.id == original_message.id:
            break

    try:
        if len(messages_to_delete) <= 100:
            async with ctx.channel.typing():
                await ctx.channel.delete_messages(messages_to_delete)
                await ctx.send(f"Deleted {len(messages_to_delete)} message(s)")
        else:
            ids = list(map(lambda m: m.id, messages_to_delete))
            async with ctx.channel.typing():
                await ctx.channel.purge(limit=len(messages_to_delete), check=lambda m: m.id in ids,
                                        bulk=False)
                await ctx.send(f"Deleted {len(messages_to_delete)} message(s)")
    except discord.Forbidden:
        await ctx.send("Architus does not have permission to remove messages in this server")
    except discord.HTTPException:
        await ctx.send("Failed to reach discord serverrs. Try again in a bit")
    except Exception:
        await ctx.send("Something went wrong. :(")


@purge.command()
async def time(ctx, time_window):
    """
    Purge a channel of messages sent in the past X amount of time

    Usage: {prefix}purge time {XXm|XXs}

    Deletes all messages in the channel in the past number of
    minutes or seconds.

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
    try:
        if time_window[2] == 'm':
            mins = int(time_window[0:2])
            diff = timedelta(minutes=mins)
        elif time_window[2] == 's':
            secs = int(time_window[0:2])
            diff = timedelta(seconds=secs)
    except ValueError:
        await ctx.send("Time value formatted improperly")
        return

    earliest = now - diff

    settings = ctx.bot.settings[ctx.guild]
    if ctx.author.id not in settings.admins_ids:
        await ctx.send("You do not have permissions to purge messaages")
        return

    try:
        messages_to_delete = await ctx.channel.history(limit=500, after=earliest).flatten()
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
        if len(messages_to_delete) <= 100:
            async with ctx.channel.typing():
                await ctx.channel.delete_messages(messages_to_delete)
                await ctx.send(f"Deleted {len(messages_to_delete)} message(s)")
        else:
            ids = list(map(lambda m: m.id, messages_to_delete))
            async with ctx.channel.typing():
                await ctx.channel.purge(limit=len(messages_to_delete), check=lambda m: m.id in ids,
                                        bulk=False)
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
