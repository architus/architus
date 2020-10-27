import discord
from discord.ext import commands

from typing import Optional


@commands.command()
async def help(ctx, command_name: Optional[str]):
    """
    Prints help messages for commands.
    """
    prefix = ctx.bot.settings[ctx.guild].command_prefix
    if command_name is None:
        msg = discord.Embed(title="Commands | Architus Docs",
                            # description="See a short help message of how to use architus commands.",
                            url="https://docs.archit.us/commands")
        msg.add_field(name="Usage",
                      value=f"```{prefix}help <command name>```View details about a command.\n\n"
                            "Click [here](https://docs.archit.us/commands) to "
                            "see a list of all the architus commands.")
    else:
        # Extract command from list of commands associated with the bot
        command = None
        for c in ctx.bot.commands:
            if c.name == command_name or command_name in c.aliases:
                command = c

        if command is None:
            await ctx.send("Could not find that command")
            return

        command_title = " ".join(command_name.split("_")).title()
        help_text = command.callback.__doc__
        usage_start = help_text.find(' ') + 1
        usage_end = help_text.find('\n')
        if usage_start == -1 or usage_start > usage_end:
            usage_start = usage_end
        usage = help_text[usage_start:usage_end]
        extra = help_text[help_text.find('\n') + 1:].replace('\n', ' ')
        try:
            url = command.callback.__doc_url__
            link = f" [more info...]({url})"
        except AttributeError:
            link = ""
            url = None
        msg = discord.Embed(title=f"{command_title} | Architus Docs",
                            # description=extra,
                            url=url)
        msg.add_field(name="Usage",
                      value=f"```{prefix}{command_name} {usage}```{extra}{link}")
    msg.colour = 0x83bdff
    msg.set_author(name="Architus Docs",
                   icon_url="https://docs.archit.us/img/logo_hex.png")
    await ctx.send(embed=msg)


def setup(bot):
    bot.add_command(help)
