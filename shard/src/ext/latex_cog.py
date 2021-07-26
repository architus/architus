# Latex command for Architus bot.
# Takes a string of latex and returns the compiled version as a png
# Inspired by: https://github.com/chuanshi/slacklatex
from discord.ext import commands
from discord.ext.commands import cooldown, BucketType
from src.utils import doc_url

from discord import File, Embed

from string import Template
import tempfile
import os

from asyncio import create_subprocess_exec, wait_for, TimeoutError
from asyncio.subprocess import DEVNULL, PIPE


class Latexify(commands.Cog, name="Latex Compiler"):

    def __init__(self, bot):
        self.bot = bot
        with open('res/generate/template.tex', 'r') as f:
            s = f.read()
        self.base_tex = Template(s)

        self.illegal_commands = [
            r"\write",
            r"\tempfile",
            r"\openout",
            r"\newwrite",
            r"\write",
            r"\input",
            r"\usepackage",
            r"\include",
            r"\def",
            r"\newcommand",
            r"\immediate",
        ]

    @commands.command(aliases=['tex'])
    @cooldown(2, 15, BucketType.user)
    @doc_url("https://docs.archit.us/commands/latex/")
    async def latex(self, ctx, *latex):
        """latex <latex code>
        Compiles latex to a png.
        """
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
                                               stdout=PIPE,
                                               stderr=DEVNULL,
                                               close_fds=True)

            try:
                stdout, _ = await wait_for(tex.communicate(), timeout=3)
            except TimeoutError:
                await ctx.send("Compilation took too long")
                tex.kill()
                return

            if not os.path.isfile(os.path.join(work_dir, 'out.dvi')):
                output = stdout.decode('UTF-8')

                # The only latex output that we want comes after the first '!'
                # Pdflatex includes a bunch of boilerplate output at the beginning
                # about the state of the environment that we don't really need.
                # In addition, the last two lines of output are just saying where
                # pdflatex stored the logs to but the discord user doesn't have
                # access to those so just get rid of them.
                error_msg = output[output.index('!'):]
                error_msg = "\n".join(error_msg.split('\n')[:-3])

                embed = Embed(title="Latex Compilation Error", description="Something went wrong")
                embed.add_field(name="Latex Code", value=latex, inline=False)
                embed.add_field(name="Compiler Error", value=error_msg, inline=False)
                await ctx.send(embed=embed)
                return

            convert = await create_subprocess_exec(
                'dvipng', '-T', 'tight', '-Q', '32', '-D', '1500', '-fg', 'rgb 1.0 1.0 1.0',
                '-bg', 'transparent', '--gamma', '100', 'out.dvi',
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
