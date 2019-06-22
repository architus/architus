import json
from discord.ext.commands import Cog

class Api(Cog):

    def __init__(self, bot):
        self.bot = bot

    async def fetch_user_dict(self, id):
        usr = await self.bot.fetch_user(int(id))
        avatar = usr.avatar_url or usr.default_avatar_url
        return json.dumps({'name': usr.name, 'avatar_url': str(avatar)})

    async def interpret(self, message):
        #print(self.bot.commands)
        return json.dumps({'response': f'looks like you said {message}'})


def setup(bot):
    bot.add_cog(Api(bot))
