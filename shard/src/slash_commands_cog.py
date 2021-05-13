
from discord.ext import commands

from aiohttp import ClientSession

from lib.config import API_ENDPOINT, client_id, secret_token, logger
from src.slash_commands.set_command import json


GUILD_TEMPLATE = API_ENDPOINT + '/applications/' + client_id + '/guilds/{guild_id}/commands'
GLOBAL_TEMPLATE = API_ENDPOINT + '/applications/' + client_id + '/commands'

headers = {
    "Authorization": f"Bot {secret_token}"
}


class Slash(commands.Cog):

    def __init__(self, bot):
        self.bot = bot

    @commands.Cog.listener()
    async def on_ready(self):

        async with ClientSession() as session:
            async with session.post(GUILD_TEMPLATE.format(guild_id=436189230390050826), headers=headers, json=json) as r:
                logger.debug(await r.text())

    async def on_socket_raw_receive(self, msg):
        logger.debug(msg)


def setup(bot):
    bot.add_cog(Slash(bot))
