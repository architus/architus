from src.commands.abstract_command import abstract_command
import discord

class purge_command(abstract_command):

    def __init__(self):
        super().__init__("purge")

    async def exec_cmd(self, **kwargs):
        settings = kwargs['settings']
        user = self.client.user
        count = 100
        channel = self.channel
        if len(self.args) > 1:
            user = self.server.get_member(self.args[1])
            if not user: return
        if len(self.args) > 2:
            count = min(int(self.args[2]), 100000)
        if len(self.args) > 3:
            if not self.message.channel_mentions: return
            channel = self.message.channel_mentions[0]
        await self.client.send_typing(self.channel)
        if (self.author.id in settings.admins_ids):
            deleted = await self.client.purge_from(channel, limit=count, check=lambda m: m.author==user)
            await self.client.send_message(self.channel, 'Deleted {} message(s)'.format(len(deleted)))
        else:
            await self.client.send_message(self.channel, 'lul %s' % self.author.mention)

        return True

    def get_help(self, **kwargs):
        return "Purge a channel of a user's messages"

    def get_usage(self):
        return "[memberid [number of messages to filter [target channel mention]]]"
