from discord.ext import commands, tasks

class MemberMessages():
    """
    Keeps track of a users most recent message history.

    Messages are just kept in a list. Every 20 seconds, the
    purge method should be called to remove messages that are
    more than 20 seconds old.
    """
    def __init__(self, mid):
        # TODO: Need info for member join/creation date.
        self.member_id = mid
        self.messages = []

    def add_message(self, m, cap):
        self.messages.append(m)
        if len(self.messages) > cap:
            return True
        return False

    def purge_old(self):
        pass

class Protecc(commands.Cog, name="Bot Net Protecc"):
    def __init__(self, bot):
        self.bot = bot
        self.member_buckets = {}

    @commands.Cog.listener()
    async def on_message(self, msg):
        pass

    @tasks.loop(seconds=20.0)
    async def purge(self):
        pass

def setup(bot):
    bot.add_cog(Protecc(bot))
