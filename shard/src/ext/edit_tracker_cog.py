from datetime import datetime
from dataclasses import dataclass
from typing import List, Dict
from contextlib import suppress

from discord.ext import commands
from discord import Embed, Message


@dataclass
class Edit:
    content: str
    timestamp: datetime


@dataclass
class EditedMsg:
    message: Message
    edits: List[Edit]


class EditTrackerCog(commands.Cog):

    def __init__(self, bot):
        self.bot = bot
        self.tracked_messages = {}  # type: Dict[int, EditedMsg]
        self.displayed_messages = {}  # type: Dict[int, Message]

    def get_edit_embed(self, msg_id):
        emsg = self.tracked_messages.get(msg_id)
        if emsg is None:
            return

        title = f"{len(emsg.edits) - 1} edit" + 's' if len(emsg.edits) > 2 else ''

        em = Embed(timestamp=emsg.message.created_at, color=0x5998FF)
        em.set_footer(text=title)
        em.set_author(name=emsg.message.author.display_name, icon_url=emsg.message.author.avatar_url)
        for edit in emsg.edits:
            delta = (edit.timestamp - emsg.edits[0].timestamp).total_seconds()
            if delta == 0.0:
                name = "original"
            else:
                name = f"+{delta:.1f}s"
            em.add_field(name=name, value=edit.content, inline=True)
        if len(em) > 6000:
            return Embed(title="Error", description="Edit tracker was too large for discord :confused:")
        return em

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        """Check if an edit dialog should be posted"""
        if user == self.bot.user:
            return
        msg = react.message
        settings = self.bot.settings[msg.guild]
        if settings.edit_emoji in str(react.emoji):
            emsg = self.tracked_messages.get(msg.id)
            if emsg and msg.id not in self.displayed_messages:
                self.displayed_messages[msg.id] = await msg.channel.send(embed=self.get_edit_embed(msg.id))

    @commands.Cog.listener()
    async def on_reaction_remove(self, react, user):
        settings = self.bot.settings[react.message.guild]
        if settings.edit_emoji in str(react.emoji) and react.count == 0:
            with suppress(KeyError):
                await self.displayed_messages.pop(react.message.id).delete()

    @commands.Cog.listener()
    async def on_message_edit(self, before, after):
        """Adds message to the store of edits"""
        if before.author == self.bot.user:
            return

        emsg = self.tracked_messages.get(before.id, EditedMsg(
                                         message=before,
                                         edits=[Edit(timestamp=datetime.now(), content=before.content)]))
        emsg.edits.append(Edit(content=after.content, timestamp=datetime.now()))
        self.tracked_messages[before.id] = emsg

        if before.id in self.displayed_messages:
            await self.displayed_messages[before.id].edit(embed=self.get_edit_embed(before.id))

    @commands.Cog.listener()
    async def on_message_delete(self, msg):
        settings = self.bot.settings[msg.guild]
        if msg.id in self.bot.deletable_messages:
            self.bot.deletable_messages.remove(msg.id)
            return
        if msg.author != self.bot.user and settings.repost_del_msg:
            em = Embed(timestamp=msg.created_at, description=msg.content, colour=0x42f468)
            em.set_author(name=msg.author.display_name, icon_url=msg.author.avatar_url)
            repost = await msg.channel.send(embed=em)
            # place the tracked edits for the deleted message on the reposted embed
            if msg.id in self.tracked_messages:
                self.tracked_messages[repost.id] = self.tracked_messages[msg.id]


def setup(bot):
    bot.add_cog(EditTrackerCog(bot))
