from src.commands.abstract_command import abstract_command
import discord

class quote_command(abstract_command):

    def __init__(self):
        super().__init__("quote")

    async def exec_cmd(self, **kwargs):
        if (self.args[1]):
            message = await self.client.get_message(self.channel, self.args[1])
            if message:
                est = self.get_datetime(message.timestamp)
                em = discord.Embed(title=est.strftime("%Y-%m-%d %I:%M %p"), description=message.content, colour=0x42f468)
                em.set_author(name=message.author.display_name, icon_url=message.author.avatar_url)
                await self.client.send_message(self.channel, embed=em)

    def get_help(self):
        return "!quote <messageid> - quotes a previous message in a pretty format"
