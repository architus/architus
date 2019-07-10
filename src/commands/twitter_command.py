import twitter
from discord.ext import commands



@commands.command(aliases=['ajax', 'masters', 'beat'])
async def ajax_masters(ctx):
    user = 'Ajaxbeat'
    id = '1143094150717304832'
    statuses = api.GetUserTimeline(user_id=id, count=1, exclude_replies=True, include_rts=False)
    msg = 'https://twitter.com/Ajaxbeat/status/' + statuses[0].id_str
    print(statuses[0].urls)
    await ctx.channel.send(msg)


def setup(bot):
    bot.add_command(ajax_masters)
