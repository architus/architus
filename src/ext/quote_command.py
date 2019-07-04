import discord
from discord.ext import commands
from pytz import timezone


@commands.command()
async def quote(ctx, message: discord.Message):
    '''Quotes a previous message in a pretty format. Use url or id.'''

    utc = message.created_at.replace(tzinfo=timezone('UTC'))
    est = utc.astimezone(timezone('US/Eastern'))
    em = discord.Embed(title=est.strftime("%Y-%m-%d %I:%M %p"), description=message.content, colour=0x42f468)
    em.set_author(name=message.author.display_name, icon_url=message.author.avatar_url)
    em.set_footer(text='#' + message.channel.name)
    try:
        if message.embeds:
            em.set_image(url=message.embeds[0].url)
        elif message.attachments:
            em.set_image(url=message.attachments[0].url)
    except (IndexError, KeyError):
        print("tried to attach image, couldn't")
    await ctx.channel.send(embed=em)


def setup(bot):
    bot.add_command(quote)
