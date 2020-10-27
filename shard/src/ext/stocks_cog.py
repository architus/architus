from discord.ext import commands
import discord
import aiohttp
import json

from lib.config import alphavantage_api_key
from src.utils import doc_url


class Stocks(commands.Cog, name="Stock Prices"):

    def __init__(self, bot):
        self.bot = bot

    @commands.command()
    @doc_url("https://docs.archit.us/commands/stocks")
    async def price(self, ctx, symbol: str):
        '''price <ticker>
        Give some daily stats about a company.'''
        async with aiohttp.ClientSession() as session:
            url = 'https://www.alphavantage.co/query?function=GLOBAL_QUOTE&'
            url += f'symbol={symbol}&apikey={alphavantage_api_key}'
            async with session.get(url) as resp:

                data = json.loads(await resp.text())
                if "Error Message" in data:
                    await ctx.send("Couldn't find that symbol")
                else:
                    data = data["Global Quote"]
                    symbol = data['01. symbol']
                    price = float(data['05. price'])
                    change = float(data['09. change'])
                    change_percent = float(data['10. change percent'][:-1])

                    em = discord.Embed(
                        title=f"{price:.2f} USD",
                        description=f"{change:.2f} ({change_percent:.2f}%) {'📈' if change > 0 else '📉'}",
                        colour=0x42f46
                    )
                    em.set_author(name=symbol)

                    await ctx.send(embed=em)


def setup(bot):
    bot.add_cog(Stocks(bot))
