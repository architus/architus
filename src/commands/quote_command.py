from src.commands.abstract_command import abstract_command
import discord

class quote_command(abstract_command):

    def __init__(self):
        super().__init__("quote")

    async def exec_cmd(self, **kwargs):
        if (self.args[1]):
            for channel in self.server.channels:
                try: message = await self.client.get_message(channel, self.args[1])
                except: message = None
                if message:
                    est = self.get_datetime(message.timestamp)
                    em = discord.Embed(title=est.strftime("%Y-%m-%d %I:%M %p"), description=message.content, colour=0x42f468)
                    em.set_author(name=message.author.display_name, icon_url=message.author.avatar_url)
                    em.set_footer(text='#'+channel.name)
                    await self.client.send_message(self.channel, embed=em)
                    return

    def get_help(self):
        return "!quote <messageid> - quotes a previous message in a pretty format"
