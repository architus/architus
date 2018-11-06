from src.commands.abstract_command import abstract_command
import random
import discord

class eight_ball_command(abstract_command):

    def __init__(self):
        super().__init__("eight_ball", aliases=['8ball', '8-ball','eightball'])

    async def exec_cmd(self, **kwargs):
        possible_responses = [
            'That is a resounding no',
            'It is not looking likely',
            'Too hard to tell',
            'It is quite possible',
            'Definitely',
            'Yep.',
            'Possibly.'
        ]
        await self.client.send_message(self.channel, random.choice(possible_responses) + ", " + self.author.mention)

    def get_help(self):
        return 'Answers a yes or no question'
    def get_usage(self):
        return "[question]"
    def get_brief(self):
        return 'Answers from the beyond'
