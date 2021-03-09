from discord.ext import commands
from lib.aiomodels import TbUserConnections


class Overwatch(commands.Cog):

    def __init__(self, bot):
        self.bot = bot
        self.tb_react_events = TbUserConnections(self.bot.asyncpg_wrapper)

    @commands.command(aliases=['overwatch'])
    async def trackoverwatch(self, ctx):
        await self.tb_react_events.select_by_id({'user_id': ctx.author.id, 'type': 'overwatch'})


def setup(bot):
    bot.add_cog(Overwatch(bot))
