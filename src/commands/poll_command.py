from src.commands.abstract_command import abstract_command
from unidecode import unidecode
import time
import re
import asyncio
import discord

class poll_command(abstract_command):

    ANSWERS = ['\N{DIGIT ZERO}\N{COMBINING ENCLOSING KEYCAP}',
    '\N{DIGIT ONE}\N{COMBINING ENCLOSING KEYCAP}',
    '\N{DIGIT TWO}\N{COMBINING ENCLOSING KEYCAP}',
    '\N{DIGIT THREE}\N{COMBINING ENCLOSING KEYCAP}',
    '\N{DIGIT FOUR}\N{COMBINING ENCLOSING KEYCAP}',
    '\N{DIGIT FIVE}\N{COMBINING ENCLOSING KEYCAP}',
    '\N{DIGIT SIX}\N{COMBINING ENCLOSING KEYCAP}',
    '\N{DIGIT SEVEN}\N{COMBINING ENCLOSING KEYCAP}',
    '\N{DIGIT EIGHT}\N{COMBINING ENCLOSING KEYCAP}',
    '\N{DIGIT NINE}\N{COMBINING ENCLOSING KEYCAP}']

    def __init__(self):
        super().__init__("poll")

    async def exec_cmd(self, **kwargs):
        reaction_callbacks = kwargs['reaction_callbacks']
        pattern = re.compile('!poll (?P<title>(?:".+")|(?:[^ ]+)) (?P<options>.*$)')
        match = pattern.search(unidecode(self.content))
        if not match: return

        self.votes = [[] for x in range(10)]
        self.options = [o.lstrip() for o in match.group('options').split(",")[:10]]
        self.title = match.group('title').replace('"', '')
        text = self.render_text(self.title, self.options, self.votes)

        self.msg = await self.channel.send(text)
        for i in range(len(self.options)):
            await self.msg.add_reaction(self.ANSWERS[i])

        reaction_callbacks[self.msg.id] = (self.on_vote_add, self.on_vote_remove)
        return True

    async def on_vote_add(self, react, user):
        try:
            i = self.ANSWERS.index(str(react.emoji))
        except ValueError:
            return
        self.votes[i].append(user)
        await self.msg.edit(content=self.render_text(self.title, self.options, self.votes))

    async def on_vote_remove(self, react, user):
        try:
            i = self.ANSWERS.index(str(react.emoji))
        except ValueError:
            return
        self.votes[i].remove(user)
        await self.msg.edit(content=self.render_text(self.title, self.options, self.votes))

    def render_text(self, title, options, votes):
        text = f"__**{title}**__\n"
        i = 0
        for option in options:
            text += "%s **%s (%d)**: %s\n" % (self.ANSWERS[i], option, len(votes[i]), ' '.join([u.mention for u in votes[i]]))
            i += 1
        return text

    def get_help(self, **kwargs):
        return "Starts a poll with some pretty formatting. Supports up to 10 options"

    def get_usage(self):
        return '"<title>" <option1>, <option2>,...'
