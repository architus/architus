from src.commands.abstract_command import abstract_command
import src.generate.letmein as letmeingen
import discord

class letmein_command(abstract_command):

    def __init__(self):
        super().__init__("letmein")

    async def exec_cmd(self, **kwargs):
        args = self.message.content.split(' ')
        del args[0]
        name = self.message.mentions[0].display_name if self.message.mentions else args[len(args) - 1]
        del args[len(args) - 1]
        letmeingen.generate(name, ' '.join(args))
        with open('res/meme.png', 'rb') as f:
            await self.client.send_file(self.channel, f, content="Here you go, " + self.author.mention)

        return True

    def get_help(self, **kwargs):
        return "stupid meme"
    def get_usage(self):
        return "<exclusion> [@member]"
