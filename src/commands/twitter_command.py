import twitter
from discord.ext import commands

api = twitter.Api(consumer_key='uhs3HzukyZvni2VU8rv5lmCk3',
                  consumer_secret='uiWwYXf7s0tiA0Xpez7FA1EyWR9gp5c7w7Pqqf6ydJt2FHCjax',
                  access_token_key='1148731389128450048-sMxNHBoXfFTYq4q3phVv8NtoyE2lE7',
                  access_token_secret='zGHswZfMncpyYyvsSJ9pkalIl4K4DHgvhjkzyYy6zx6fk')


@commands.command(aliases=['ajax', 'masters', 'beat'])
async def ajax_masters(ctx):
    #user = 'Ajaxbeat'
    id = '1143094150717304832'
    statuses = api.GetUserTimeline(user_id=id, count=1, exclude_replies=True, include_rts=False)
    msg = 'https://twitter.com/Ajaxbeat/status/' + statuses[0].id_str
    print(statuses[0].urls)
    await ctx.channel.send(msg)


def setup(bot):
    bot.add_command(ajax_masters)
