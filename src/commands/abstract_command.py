import discord
from discord import ChannelType
from pytz import timezone
import pytz
 
class abstract_command():
 
    def __init__(self, name, aliases=[]): 
        self.name = name
        self.aliases = aliases
        super().__init__()

    async def execute(self, message, client, **kwargs):
        self.client = client
        self.message = message
        self.content = self.message.content
        self.args = self.message.clean_content.split(' ')
        self.channel = self.message.channel
        self.server = self.message.channel.guild
        self.author = self.message.author
        return await self.exec_cmd(**kwargs)
   
    def get_aliases(self):
        return [self.name] + self.aliases

    def format_help(self, invocation, **kwargs):
        return "```usage:          %s %s\ndescription:    %s```" % (invocation, self.get_usage(), self.get_help(**kwargs))

    def get_help(self, **kwargs):
        raise NotImplementedError

    def get_usage(self):
        raise NotImplementedError

    def get_brief(self):
        return self.get_help()

    async def exec_cmd(self, **kwargs):
        raise NotImplementedError

    def get_datetime(self, timestamp):
        utc = timestamp.replace(tzinfo=timezone('UTC'))
        est = utc.astimezone(timezone('US/Eastern'))
        return est

    def get_custom_emoji(self, server, emojistr):
        for emoji in server.emojis:
            if emoji.name == emojistr:
                return emoji
        raise Exception('no emoji of name "%s" the server' % emojistr)
        return None
