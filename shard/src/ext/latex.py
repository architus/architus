from discord.ext import commands
import requests
import discord
import aiohttp
import shutil
import io
import PIL
import PIL.Image
from open_relative import *


class Latex(commands.Cog, name="LaTeX Renderer"):

    HOST = 'http://rtex.probablyaweb.site/api/v2'
    DARK_MODE_TEXT_COLOR = "F0F0F0"

    def load_template(self):
        with open_relative('../../latex_template.txt', encoding='utf-8') as f:
            raw = f.read()
        return raw

    TEMPLATE = load_template()

    def __init__(self, bot):
        self.bot = bot

    def download_file(self, url, dest_filename):
        response = requests.get(url, stream=True)
        response.raise_for_status()
        with open(dest_filename, 'wb') as out_file:
            shutil.copyfilobj(response.raw, out_file)

    async def render(self, ctx, latex: str):
        '''
        Render some LaTeX code and post the result as an image.
        '''
        latex_file = TEMPLATE.replace("#TEXTCOLOR",Latex.DARK_MODE_TEXT_COLOR).replace("#CONTENT",latex)
        payload = {
            'code': latex_file,
            'format': 'png'
        }
        async with aiohttp.ClientSession() as session:
            try:
                async with session.post(Latex.HOST, json=payload, timeout=8) as loc_req:
                    loc_req.raise_for_status()
                    jdata = await loc_req.json()
                    if jdata['status'] == 'error':
                        print("jdata has error status")
                        await ctx.send('Failed to render LaTeX.')
                    filename = jdata['filename']
                async with session.get(f"{Latex.HOST}/{filename}", json=payload, timeout=8) as img_req:
                    img_req.raise_for_status()
                    fo = io.BytesIO(await img_req.read())
                    image = PIL.Image.open(fo).convert('RGBA')
            except aiohttp.client_exceptions.ClientResponseError:
                print("ClientResponseError from Latex render method")
                await ctx.send('Failed to render LaTeX.')
        if image.width <= 2 or image.height <= 2:
            raise Exception("Rendering Error")
            print("Rendering Error from Latex render method")
        OVERSAMPLING = 2
        border_size = 5 * OVERSAMPLING
        colour_back = '36393E'
        colour_back = (
            int(colour_back[0:2], base=16),
            int(colour_back[2:4], base=16),
            int(colour_back[4:6], base=16)
        )
        width, height = image.size
        backing = PIL.Image.new('RGBA', (width + border_size * 2, height + border_size * 2), colour_back)
        backing.paste(image, (border_size, border_size), image)
        if OVERSAMPLING != 1:
            backing = backing.resize((backing.width // OVERSAMPLING, backing.height // OVERSAMPLING),
                                     resample=PIL.Image.BICUBIC)
        fobj = io.BytesIO()
        backing.save(fobj, format='PNG')
        fobj = io.BytesIO(fobj.getvalue())
        return fobj

    @commands.command()
    async def latex(self, ctx, *args):
        content = ' '.join(args)
        image = await self.render(ctx, content)
        await ctx.send(file=discord.File(image, 'latex.png'))


def setup(bot):
    bot.add_cog(Latex(bot))
