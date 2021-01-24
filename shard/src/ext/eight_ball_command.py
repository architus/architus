import random
from discord.ext import commands
from src.utils import doc_url


@commands.command(aliases=['8ball', '8-ball', 'eightball'])
@doc_url("https://docs.archit.us/commands/8ball/")
async def eight_ball(ctx):
    '''8ball
    Answers from the beyond.
    '''
    possible_responses = [
        'That is a resounding no',
        'It is not looking likely',
        'Too hard to tell',
        'It is quite possible',
        'Definitely',
        'Yep',
        'Possibly'
    ]
    await ctx.send(random.choice(possible_responses) + ", " + ctx.author.mention)


def setup(bot):
    bot.add_command(eight_ball)
