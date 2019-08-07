import twitter
from discord.ext import commands
from lib.config import twitter_consumer_key
from lib.config import twitter_consumer_secret
from lib.config import twitter_access_token_key
from lib.config import twitter_access_token_secret


api = twitter.Api(consumer_key=twitter_consumer_key,
                  consumer_secret=twitter_consumer_secret,
                  access_token_key=twitter_access_token_key,
                  access_token_secret=twitter_access_token_secret)


@commands.command(aliases=['ajax', 'masters', 'beat'])
async def ajax_masters(ctx):
    '''Tells you if Ajax is in masters or not'''
    id = '1143094150717304832'
    statuses = api.GetUserTimeline(user_id=id, count=1, exclude_replies=True, include_rts=False)
    msg = 'https://twitter.com/Ajaxbeat/status/' + statuses[0].id_str
    await ctx.channel.send(msg)


def setup(bot):
    bot.add_command(ajax_masters)
