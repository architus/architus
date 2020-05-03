# Latex command for Architus bot.
# Takes a string of latex and returns the compiled version as a png
# Inspired by: https://github.com/chuanshi/slacklatex
from discord.ext import commands
from discord.ext.commands import cooldown, BucketType
from discord import File

from string import Template
import tempfile
import os
from asyncio import create_subprocess_exec, wait_for, TimeoutError
from asyncio.subprocess import DEVNULL


class Latexify(commands.Cog, name="Latex Compiler"):

    def __init__(self, bot):
        self.bot = bot
        with open('res/generate/template.tex', 'r') as f:
            s = f.read()
        self.base_tex = Template(s)

        self.illegal_commands = [
            "\\write",
            "\\tempfile",
            "\\openout",
            "\\newwrite",
            "\\write",
            "\\input",
            "\\usepackage",
            "\\include"
        ]

    @commands.command(aliases=['tex'])
    @cooldown(2, 15, BucketType.user)
    async def latex(self, ctx, *latex):
        for l in latex:
            for c in self.illegal_commands:
                if l.find(c) != -1:
                    await ctx.send("Illegal latex command")
                    return
        latex = " ".join(latex)
        with tempfile.TemporaryDirectory() as work_dir:
            out_txt = self.base_tex.substitute(my_text=latex)

            with open(os.path.join(work_dir, 'out.tex'), 'w') as f:
                f.write(out_txt)

            tex = await create_subprocess_exec('latex', '-halt-on-error', '-no-shell-escape',
                                               '-interaction batchmode', 'out.tex',
                                               cwd=work_dir,
                                               stdout=DEVNULL,
                                               stderr=DEVNULL,
                                               close_fds=True)
            try:
                await wait_for(tex.wait(), timeout=3)
            except TimeoutError:
                await ctx.send("Compilation took too long")
                return

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
    bot.add_cog(Latexify(bot))
