import discord
from discord.ext import commands

from lib.config import logger
from src.utils import doc_url


@commands.command()
@doc_url("https://docs.archit.us/commands/quote")
async def quote(ctx, message: discord.Message):
    '''quote <message url|message id>
    Quotes a previous message in a pretty format. Use url or id.'''

    em = discord.Embed(
        description=message.content,
        timestamp=message.created_at,
        colour=0x42f468)
    em.set_author(name=message.author.display_name, icon_url=message.author.avatar_url)
    em.set_footer(text='#' + message.channel.name)
    try:
        if message.embeds:
            em.set_image(url=message.embeds[0].url)
        elif message.attachments:
            em.set_image(url=message.attachments[0].url)
    except (IndexError, KeyError):
        logger.exception("tried to attach image, couldn't")
    await ctx.channel.send(embed=em)


def setup(bot):
    bot.add_command(quote)
