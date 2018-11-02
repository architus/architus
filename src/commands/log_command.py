from src.commands.abstract_command import abstract_command
from src.list_embed import list_embed, dank_embed
import enchant
from discord import ChannelType
import time
import re
import discord

class log_command(abstract_command):
    def __init__(self):
        super().__init__("log")

    async def exec_cmd(self, **kwargs):
        channel = self.channel
        await self.client.send_typing(channel)
        msgs = []
        do_filter = bool(self.message.mentions)
        try:
            num = int(re.search(r'\d+', self.message.clean_content).group())
        except:
            num = 25
        num = max(num, 1)
        num = min(num, 200)

        async for message in self.client.logs_from(channel, limit=5000):
            if (not do_filter or message.author in self.message.mentions):
                msgs.append(message)
            if (len(msgs) >= num):
                break
        msgs.reverse()
        twenty_five = [msgs[x:x+25] for x in range(0, len(msgs), 25)]
        target_channel = channel
        if len(twenty_five) > 1:
            botcommands = discord.utils.get(channel.server.channels, name='bot-commands', type=ChannelType.text)
            if botcommands:
                target_channel = botcommands
                await self.client.send_message(channel, botcommands.mention)

        for messages in twenty_five:
            lembed = list_embed('Last %s messages' % num, channel.mention, self.client.user)
            lembed.color = 0x9e6338
            lembed.icon_url = 'https://engineeringblog.yelp.com/images/previews/logfeeder.png'
            lembed.name = channel.server.name
            for message in messages:
                if message.id == self.message.id:
                    continue
                elif message.content:
                    lembed.add(message.author.display_name, message.content)
                elif message.embeds:
                    em = message.embeds[0]
                    lembed.add(message.author.display_name, em['url'] if 'url' in em.keys() else em['title'])
                elif message.attachments:
                    lembed.add(message.author.display_name, message.attachments[0]['url'] or '')
            await self.client.send_message(target_channel, embed=lembed.get_embed())
    def get_help(self):
        return "!spellcheck"

    def get_usage(self):
        return "!spellcheck"
