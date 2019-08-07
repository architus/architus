from discord.ext import commands
from contextlib import suppress


@commands.command()
async def purge(ctx, *args):
    '''
    Purge a channel of a user's messages
    optionally include member id, channel, or message limit
    '''
    settings = ctx.bot.settings[ctx.guild]
    user = ctx.bot.user

    for arg in args:
        with suppress(ValueError):
            user = ctx.guild.get_member(int(arg))
            if user:
                break
    else:
        user = ctx.bot.user

    for arg in args:
        with suppress(ValueError):
            if int(arg) < 100000:
                count = int(arg)
    else:
        count = 100

    channel = ctx.channel
    if ctx.message.channel_mentions:
        channel = ctx.message.channel_mentions[0]

    async with ctx.channel.typing():
        if (ctx.author.id in settings.admins_ids):
            deleted = await channel.purge(limit=count, check=lambda m: m.author == user)
            await ctx.send('Deleted {} message(s)'.format(len(deleted)))
        else:
            await ctx.send(f'lul {ctx.author.mention}')


def setup(bot):
    bot.add_command(purge)
