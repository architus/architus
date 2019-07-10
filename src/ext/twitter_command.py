import twitter
from discord.ext import commands

try:
    lines = [line.rstrip('\n') for line in open('.secret_token')]
    consumer_key = lines[5]
    consumer_secret = lines[6]
    access_token_key = lines[7]
    access_token_secret = lines[8]
except Exception as e:
    print(e)
    print('error reading .secret_token, make it you aut')


api = twitter.Api(consumer_key=consumer_key,
                  consumer_secret=consumer_secret,
                  access_token_key=access_token_key,
                  access_token_secret=access_token_secret)


@commands.command(aliases=['ajax', 'masters', 'beat'])
async def ajax_masters(ctx):
    '''Tells you if Ajax is in masters or not'''
    id = '1143094150717304832'
    statuses = api.GetUserTimeline(user_id=id, count=1, exclude_replies=True, include_rts=False)
    msg = 'https://twitter.com/Ajaxbeat/status/' + statuses[0].id_str
    await ctx.channel.send(msg)


def setup(bot):
    bot.add_command(ajax_masters)
