# Latex command for Architus bot.
# Takes a string of latex and returns the compiled version as a png
# Inspired by: https://github.com/chuanshi/slacklatex
from discord.ext import commands
from discord import File

from string import Template
import tempfile
import subprocess
import os


@commands.command(aliases=['tex'])
async def latex(ctx, latex):
    with tempfile.TemporaryDirectory() as work_dir:
        with open('src/ext/template.tex', 'r',) as f:
            s = Template(f.read())
        out_txt = s.substitute(my_text=latex)

        with open(os.path.join(work_dir, 'out.tex'), 'w') as f:
            f.write(out_txt)

        subprocess.run(['pdflatex', '-halt-on-error', 'out.tex'],
                       cwd=work_dir, stdout=None, stderr=None)

        if not os.path.isfile(os.path.join(work_dir, 'out.pdf')):
            await ctx.send("Compilation failed")
            return

        subprocess.run(['convert', '-density', '300', 'out.pdf',
                        '-quality', '100', '-sharpen', '0x1.0',
                        'out.png'], cwd=work_dir, stdout=None,
                       stderr=None)
        if not os.path.isfile(os.path.join(work_dir, 'out.png')):
            await ctx.send("Image conversion failed")
            return

        f = File(os.path.join(work_dir, 'out.png'))
        await ctx.send(file=f)


def setup(bot):
    bot.add_command(latex)
