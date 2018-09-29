from discord import ChannelType
from src.commands.abstract_command import abstract_command
import src.threed as spectrum_gen
import discord

class spectrum_threed_command(abstract_command):

    def __init__(self):
        super().__init__("spectrum_3d")

    async def exec_cmd(self, **kwargs):
        self.karma_dict = kwargs['karma_dict']
        await self.client.send_typing(self.channel)
        x = []
        y = []
        z = []
        names = []
        for mem_id in self.karma_dict:
            member = self.server.get_member(mem_id)
            if (member is not None) :
                names.append(member.display_name)
                toxic = self.get_toxc_percent(mem_id)
                nice = self.get_nice_percent(mem_id)
                aut = self.get_autism_percent(mem_id)
                norm = self.get_normie_percent(mem_id)
                z.append(0)
                if (toxic > nice):
                    x.append(-1*(toxic) / 10)
                else:
                    x.append(nice / 10)
                if (norm > aut):
                    y.append(-1*(norm) / 10)
                else:
                    y.append(aut / 10)
            #y.append((get_autism_percent(member) - get_normie_percent(member)) / 10)
        spectrum_gen.generate(x, y, z, names)
        with open('res/foo.webm', 'rb') as f:
            await self.client.send_file(self.channel, f, content="Here you go, " + self.author.mention)

    def get_help(self):
        return "!spectrum - generate a graph of autism\nVote :pech: for toxic, 🅱️for autistic, ❤ for nice, and :reee: for normie." ,

    def get_usage(self):
        return "!spectrum"


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
