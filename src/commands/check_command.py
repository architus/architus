from src.commands.abstract_command import abstract_command
from src.models import User
import discord

class check_command(abstract_command):

    def __init__(self):
        super().__init__("check")

    async def exec_cmd(self, **kwargs):
        self.karma_dict = kwargs['karma_dict']
        self.session = kwargs['session']
        await self.client.send_typing(self.channel)
        for member in self.message.mentions:
            if (member == self.client.user):
                await self.client.send_message(self.channel, "Leave me out of this, " + self.author.mention)
                return True
            if (member.id not in self.karma_dict):
                self.karma_dict[member.id] = [2,2,2,2,0]
                new_user = User(member.id, self.karma_dict[member.id])
                self.session.add(new_user)
            response = member.display_name + " is "
            response += ("{:3.1f}% autistic".format(self.get_autism_percent(member.id)) if (self.get_autism_percent(member.id) >= self.get_normie_percent(member.id)) else "{:3.1f}% normie".format(self.get_normie_percent(member.id)))
            response += " and " + ("{:3.1f}% toxic".format(self.get_toxc_percent(member.id)) if (self.get_toxc_percent(member.id) >= self.get_nice_percent(member.id)) else "{:3.1f}% nice".format(self.get_nice_percent(member.id)))
            response += " with %d bots." % self.karma_dict[member.id][4]
            await self.client.send_message(self.channel, response)
        return bool(self.message.mentions)

    def get_help(self, **kwargs):
        return "See exactly how autistic people are"
    def get_usage(self):
        return "<@member>..."
    def get_brief(self):
        return 'Check up on a member'

    def get_autism_percent(self, m):
        if (self.karma_dict[m][0] + self.karma_dict[m][1] == 0):
            return 
        return ((self.karma_dict[m][0] - self.karma_dict[m][1]) / (self.karma_dict[m][0] + self.karma_dict[m][1])) * 100
    def get_normie_percent(self, m):
        if (self.karma_dict[m][0] + self.karma_dict[m][1] == 0):
            return 0
        return ((self.karma_dict[m][1] - self.karma_dict[m][0]) / (self.karma_dict[m][1] + self.karma_dict[m][0])) * 100
    def get_nice_percent(self, m):
        if (self.karma_dict[m][2] + self.karma_dict[m][3] == 0):
            return 0
        return ((self.karma_dict[m][2] - self.karma_dict[m][3]) / (self.karma_dict[m][2] + self.karma_dict[m][3])) * 100
    def get_toxc_percent(self, m):
        if (self.karma_dict[m][2] + self.karma_dict[m][3] == 0):
            return 0
        return ((self.karma_dict[m][3] - self.karma_dict[m][2]) / (self.karma_dict[m][3] + self.karma_dict[m][2])) * 100
