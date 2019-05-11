from src.commands.abstract_command import abstract_command
from discord import ChannelType
import discord

class messagecount_command(abstract_command):

    def __init__(self):
        super().__init__("messagecount")

    async def exec_cmd(self, **kwargs):
        ctxchannel = self.channel
        cache = kwargs['cache']
        cache[ctxchannel.server].setdefault('messages', {})
        await self.client.send_typing(ctxchannel)
        blacklist = []
        words = 0
        messages = 0
        victim = self.message.mentions[0]
        for channel in self.server.channels:
            try:
                await self.client.send_typing(ctxchannel)
                if not channel in blacklist and channel.type == ChannelType.text:
                    if not channel in cache[ctxchannel.server]['messages'].keys() or not cache[ctxchannel.server]['messages'][channel]:
                        print("reloading cache for " + channel.name)
                        iterator = [log async for log in self.client.logs_from(channel, limit=1000000)]
                        logs = list(iterator)
                        cache[ctxchannel.server]['messages'][channel] = logs
                    msgs = cache[ctxchannel.server]['messages'][channel]
                    for msg in msgs:
                        if msg.author == victim:
                            messages += 1
                            words += len(msg.clean_content.split())
            except Exception as e: print(e)
        await self.client.send_message(ctxchannel, "%s has sent %d words across %d messages" % (victim.display_name, words, messages))

        return True

    def get_help(self, **kwargs):
        return "Count the total messages a user has sent in the server"

    def get_usage(self):
        return "[@user]"
