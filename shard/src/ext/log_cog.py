from datetime import datetime
from discord.ext import commands
from lib.models import Log
import json


class LogCog(commands.Cog):

    MSG_EDIT = 'msg_edit'
    MSG_ADD = 'msg_add'
    MSG_DEL = 'msg_del'

    def __init__(self, bot):
        self.bot = bot
        self.session = self.bot.session

    def insert_row(self, guild_id, type, content, user_id, message_id=None, timestamp=None):
        timestamp = timestamp or datetime.now()
        log = Log(guild_id, type, content, user_id, message_id, timestamp)
        self.session.add(log)
        self.session.commit()

    @commands.Cog.listener()
    async def on_message_edit(self, before, after):
        if before.content == after.content or before.author.id == self.bot.user.id:
            return
        self.insert_row(
            before.channel.guild.id,
            LogCog.MSG_EDIT,
            json.dumps({
                'before': before.content,
                'after': after.content,
            }),
            before.author.id,
            before.id,
        )

    @commands.Cog.listener()
    async def on_message(self, msg):
        pass

    @commands.Cog.listener()
    async def on_message_delete(self, msg):
        self.insert_row(
            msg.channel.guild.id,
            LogCog.MSG_DEL,
            json.dumps({
                'content': msg.content,
            }),
            None,
            msg.id,
        )

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        pass

    @commands.Cog.listener()
    async def on_reaction_remove(self, react, user):
        pass


def setup(bot):
    bot.add_cog(LogCog(bot))
