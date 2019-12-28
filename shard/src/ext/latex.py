from discord.ext import commands
import discord
import requests
import aiohttp
import shutil
import io
import PIL
import PIL.Image

class Latex(commands.Cog, name="LaTeX Renderer"):

    HOST = 'http://rtex.probablyaweb.site/api/v2'

    def __init__(self, bot):
        self.bot = bot

    def download_file(self, url, dest_filename):
        response = requests.get(url, stream = True)
        response.raise_for_status()
        with open(dest_filename, 'wb') as out_File:
            shutil.copyfilobj(response.raw, out_file)

    async def render(self, ctx, latex: str):
        '''
        Render some LaTeX code and post the result as an image.
        '''
        latex_file = (
            f"\\documentclass{{article}}\n"
            f"\\begin{{document}}\n"
            f"\\pagenumbering{{gobble}}\n"
            f"\\[{latex}\\]\n"
            f"\\end{{document}}\n"
        )
        payload = {
            'code': latex,
            'format': 'png'
        }
        async with aiohttp.ClientSession() as session:
            try:
                async with session.post(HOST, json=payload, timeout=8) as loc_req:
                    loc_req.raise_for_status()
                    jdata = await loc_req.json()
                    if jdata['status'] == 'error':
                        await ctx.send('Failed to render LaTeX.')
                    filename = jdata['filename']
                async with session.get(f"{HOST}/{filename}", json=payload, timeout=8) as img_req:
                    img_re.raise_for_status()
                    fo = io.BytesIO(await img_req.read())
                    image = PIL.Image.open(fo).convert('RGBA')
            except aiohttp.client_exceptions.ClientResponseError:
                await ctx.send('Failed to render LaTeX.')
        if image.width <= 2 or image.height <= 2:
            raise RenderingError(None)
        border_size = 5 * OVERSAMPLING
        colour_back = imageutil.hex_to_tuple(colour_back)
        width, height = image.size
        backing = imageutil.new_monocolour((width + border_size * 2, height + border_size * 2), colour_back)
        backing.paste(image, (border_size, border_size), image)
        if OVERSAMPLING != 1:
            backing = backing.resize((backing.width // OVERSAMPLING, backing.height // OVERSAMPLING), resample = PIL.Image.BICUBIC)
        fobj = io.BytesIO()
        backing.save(fobj, format='PNG')
        fobj = io.BytesIO(fobj.getvalue())
        return fobj

    @commands.command()
    async def latex(self, ctx, content: str):
        image = await render(ctx, content)
        ctx.send(file=Discord.file(image, 'latex.png'))

def setup(bot):
    bot.add_cog(Latex(bot))
