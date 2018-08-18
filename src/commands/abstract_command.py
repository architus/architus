from abc import ABC, abstractmethod
import discord
from pytz import timezone
import pytz
 
class abstract_command(ABC):
 
    def __init__(self, name, aliases=[]): 
        self.name = name
        self.aliases = aliases
        super().__init__()

    async def execute(self, context, client, **kwargs):
        self.context = context
        self.client = client
        self.message = context.message
        self.content = context.message.content
        self.args = self.content.split(' ')
        self.channel = context.message.channel
        self.server = context.message.channel.server
        self.author = context.message.author
        await self.exec_cmd(**kwargs)
   
    def get_aliases(self):
        return [self.name] + aliases

    @abstractmethod
    def get_help(self):
        pass

    @abstractmethod
    async def exec_cmd(self, **kwargs):
        pass

    def get_datetime(self, timestamp):
        utc = timestamp.replace(tzinfo=timezone('UTC'))
        est = utc.astimezone(timezone('US/Eastern'))
        return est
