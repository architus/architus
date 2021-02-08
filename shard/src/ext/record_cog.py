"""
Command for setting privacy preferences for recording.
"""

from discord.ext import commands


@commands.command(aliases=['unmute_me'])
async def mute_me(ctx):
    """
    Adds the user to a list of people that will not be recorded.
    Run again to toggle setting.
    """

    settings = ctx.bot.settings[ctx.guild]
    excludes = settings.voice_exclude
    author = ctx.author
    if author.id in excludes:
        excludes.remove(author.id)
        await ctx.send(f"{author.display_name} will now be included in voice recordings")
    else:
        excludes.append(author.id)
        await ctx.send(f"{author.display_name} will not be included in voice recordings")
    settings.voice_exclude = excludes


def setup(bot):
    """
    Add cog to bot.
    """
    bot.add_command(mute_me)
