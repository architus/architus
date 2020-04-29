# Architus bot command for getting the latest and greatest webcomics
# sent directly to your discord channel.

from discord.ext import commands
from discord import Embed
from bs4 import BeautifulSoup as soup
import aiohttp


# Set function up as a bot command with two other aliases.
# Command called by sending [prefix]webcomic [comic] in a channel.
# webcomic could also be any of the aliases listed below.
# [comic] must a comic from the supported comics list seen in the
# function. The supported comics can be listed by the command
# [prefix]webcomic list
@commands.command(aliases=['wc', 'comic'])
async def webcomic(ctx, comic):
    # list of the supported comics
    comics = ['xkcd', 'smbc']

    # send list of supoorted comics
    if comic.lower() == "list":
        msg = "Architus currently supports the following comics: " + ", ".join(comics)
        await ctx.channel.send(msg)
        return

    # Send an error message if invalid comic request
    if comic.lower() not in comics:
        await ctx.channel.send(f"Architus does not support the {comic} webcomic")
        return

    # Send an XKCD embed to the channel
    if comic.lower() == 'xkcd':
        async with aiohttp.ClientSession() as session:
            # Uses XKCD's json api to get the latest comic
            async with session.get("https://xkcd.com/info.0.json") as resp:
                data = await resp.json()
                em = Embed(title="Today's XKCD Comic",
                           description=data['safe_title'],
                           url="https://xkcd.com")
                em.set_image(url=data['img'])
                em.color = 0x7b8fb7
                em.set_footer(text=data['alt'])

                await ctx.channel.send(embed=em)
                return

    # Send SMBC comic to the channel
    if comic.lower() == 'smbc':
        async with aiohttp.ClientSession(headers={'connection': 'close'}) as session:
            # Simply scrapes the SMBC home page for necessary data
            async with session.get("https://smbc-comics.com") as resp:
                text = await resp.text()
                page = soup(text, 'html.parser')
                t = page.title.text.split(" - ")[1]
                em = Embed(title="Today's SMBC Comic",
                           description=t,
                           url="https://smbc-comics.com")
                em.set_image(url=page.find(id='cc-comic')['src'])
                em.color = 0x7b8fb7
                em.set_footer(text=page.find(id='cc-comic')['title'])

                await ctx.channel.send(embed=em)


def setup(bot):
    bot.add_command(webcomic)
