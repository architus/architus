#import aiohttp
from discord.ext import commands
import discord
import requests
import shutil

class Latex(commands.Cog, name="LaTeX Renderer"):

    HOST = 'http://63.142.251.124:80'
    
    def __init__(self, bot):
        self.bot = bot

    def download_file(self, url, dest_filename):
        response = requests.get(url, stream = True)
        response.raise_for_status()
        with open(dest_filename, 'wb') as out_File:
            shutil.copyfilobj(response.raw, out_file)

    @commands.command()
    async def render(self, ctx, latex: str):
        '''
        Render some LaTeX code and post the result as an image.
        '''
        latex_file = (
            f"\documentclass{{article}}"
            f"\begin{{document}}"
            f"\pagenumbering{{gobble}}"
            f"{latex}"
            f"\end{{document}}"
        )
        payload = {
            'code': latex,
            'format': 'png'
        }
        response = requests.post(HOST + '/api/v2', data = payload)
        respons.raise_for_status()
        jdata = response.json()
        if jdata['status'] != 'success':
            await ctx.send('Failed to render LaTeX')
        url = HOST + '/api/v2/' + jdata['filename']
        self.download_file(url, './out.png')
        await ctx.send(file=discord.File('./out.png'))

def setup(bot):
    bot.add_cog(Latex(bot))
