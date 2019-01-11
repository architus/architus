from src.commands.abstract_command import abstract_command
from discord import ChannelType
import time
import re
import discord
import json
LINHS_ID = '81231616772411392'

class spellcheck_command(abstract_command):
    def __init__(self):
        super().__init__("spellcheck")

    async def exec_cmd(self, **kwargs):
        cache = kwargs['cache']
        ctxchannel = self.channel
        cache[ctxchannel.server].setdefault('messages', {})
        await self.client.send_typing(self.channel)
        blacklist = []
        blacklist.append(discord.utils.get(self.server.channels, name='bot-commands', type=ChannelType.text))
        blacklist.append(discord.utils.get(self.server.channels, name='private-bot-commands', type=ChannelType.text))
        with open('res/words/words.json') as f:
            d = json.loads(f.read())
        correct_words = 0
        words = 1
        victim = self.message.mentions[0]
        for channel in self.server.channels:
            try:
                await self.client.send_typing(self.channel)
                if not channel in blacklist and channel.type == ChannelType.text:
                    if not channel in cache[ctxchannel.server]['messages'].keys() or not cache[ctxchannel.server]['messages'][channel]:
                        print("reloading cache for " + channel.name)
                        iterator = [log async for log in self.client.logs_from(channel, limit=7500)]
                        logs = list(iterator)
                        cache[ctxchannel.server]['messages'][channel] = logs
                    msgs = cache[ctxchannel.server]['messages'][channel]
                    for msg in msgs:
                        if msg.author == victim:
                            for word in msg.clean_content.split():
                                if word[0] == '!': continue
                                words += 1
                                if word in d and len(word) > 1 or word in ['a','i', 'A', 'I']:
                                    correct_words += 1
            except Exception as e:
                print(e)
        linh_modifier = 10 if victim.id == LINHS_ID else 0
        await self.client.send_message(self.channel, "%.1f%s out of the %d scanned words sent by %s are spelled correctly" %
                (((correct_words/words)*100) - linh_modifier, '%', words, victim.display_name))


    def get_help(self):
        return "Checks the spelling of the user mentioned"

    def get_usage(self):
        return "<@member>"
