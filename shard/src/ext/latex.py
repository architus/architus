# Latex command for Architus bot.
# Takes a string of latex and returns the compiled version as a png
# Inspired by: https://github.com/chuanshi/slacklatex
from discord.ext import commands
from discord import File

from string import Template
import tempfile
import os
from asyncio import create_subprocess_exec
from asyncio.subprocess import DEVNULL


@commands.command(aliases=['tex'])
async def latex(ctx, latex):
    with tempfile.TemporaryDirectory() as work_dir:
        with open('src/ext/template.tex', 'r',) as f:
            s = Template(f.read())
        out_txt = s.substitute(my_text=latex)

        with open(os.path.join(work_dir, 'out.tex'), 'w') as f:
            f.write(out_txt)

        tex = await create_subprocess_exec('latex', '-halt-on-error', 'out.tex',
                                           cwd=work_dir,
                                           stdout=DEVNULL,
                                           stderr=DEVNULL,
                                           close_fds=True)
        await tex.wait()

        if not os.path.isfile(os.path.join(work_dir, 'out.dvi')):
            await ctx.send("Compilation failed")
            return

        convert = await create_subprocess_exec('dvipng', '-T', 'tight', '-D', '300', 'out.dvi',
                                               cwd=work_dir,
                                               stdout=DEVNULL,
                                               stderr=DEVNULL,
                                               close_fds=True)
        await convert.wait()

        if not os.path.isfile(os.path.join(work_dir, 'out1.png')):
            await ctx.send("Image conversion failed")
            return

        f = File(os.path.join(work_dir, 'out1.png'))
        await ctx.send(file=f)


def setup(bot):
    bot.add_command(latex)
