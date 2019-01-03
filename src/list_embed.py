import discord
class list_embed:
    def __init__(self, title, description, author=None):
        self.title = title
        self.description = description
        self.list_items = []
        self.author = author
        if self.author:
            self.name = self.author.display_name
            self.icon_url = author.avatar_url
        else:
            self.name = title
            self.icon_url = ''
        self.color = 0x5998ff

    def add(self, title, body):
        self.list_items.append((title, body))
     
    def get_embed(self):
        em = discord.Embed(title=self.title, description=self.description, colour=self.color)
        em.set_author(name=self.name, icon_url=self.icon_url)
        num = 230 if len(self.list_items) > 10 else 460
        
        for tup in self.list_items:
            mod_bod = (tup[1][:num-3] + '...') if len(tup[1]) > num-3 else tup[1]
            em.add_field(name=tup[0], value=mod_bod, inline=True)
        return em

class dank_embed:
    def __init__(self, embed_dict):
        self.embed = embed_dict

    def to_dict(self):
        return self.embed

