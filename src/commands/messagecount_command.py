from src.commands.abstract_command import abstract_command
import random, string, os
import src.generate.wordcount as wordcount_gen
from discord import ChannelType
import discord
IMAGE_CHANNEL_ID = '577523623355613235'

class messagecount_command(abstract_command):

    def __init__(self):
        super().__init__("messagecount")

    async def exec_cmd(self, **kwargs):
        ctxchannel = self.channel
        cache = kwargs['cache']
        cache[ctxchannel.server].setdefault('messages', {})
        await self.client.send_typing(ctxchannel)
        blacklist = []
        word_counts = {}
        message_counts = {}
        victim = self.message.mentions[0] if self.message.mentions else None
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
                        message_counts[msg.author] = (message_counts[msg.author] if msg.author in message_counts else 0) + 1
                        word_counts[msg.author] = (word_counts[msg.author] if msg.author in word_counts else 0) + len(msg.clean_content.split())
                        #if msg.author == victim:
                            #messages += 1
                            #words += len(msg.clean_content.split())
            except Exception as e: print(e)
        #await self.client.send_message(ctxchannel, "%s has sent %d words across %d messages" % (victim.display_name, word_counts[victim], message_counts[victim]))

        key = ''.join([random.choice(string.ascii_letters) for n in range(10)])
        wordcount_gen.generate(key, message_counts, word_counts, victim)
        with open('res/word%s.png' % key, 'rb') as f:
            channel = discord.utils.get(self.client.get_all_channels(), id=IMAGE_CHANNEL_ID)

            msg = await self.client.send_file(channel, f)

        em = discord.Embed(title="Top 5 Message Senders", description=self.server.name)
        em.set_image(url=msg.attachments[0]['url'])
        em.color = 0x7b8fb7
        if victim:
            em.set_footer(text="{0} has sent {1:,} words across {2:,} messages".format(victim.display_name, word_counts[victim], message_counts[victim]), icon_url=victim.avatar_url)

        await self.client.send_message(self.channel, embed=em)

        os.remove("res/word%s.png" % key)




        return True

    def get_help(self, **kwargs):
        return "Count the total messages a user has sent in the server"

    def get_usage(self):
        return "[@user]"
