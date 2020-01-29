from datetime import datetime

from discord.ext import commands
import discord
from src.smart_message import smart_message
from src.utils import timezone_aware_format


class EditTrackerCog(commands.Cog):

    def __init__(self, bot):
        self.bot = bot
        self.tracked_messages = {}

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        """Check if an edit dialog should be posted"""
        if user == self.bot.user:
            return
        settings = self.bot.settings[react.message.guild]
        if settings.edit_emoji in str(react.emoji):
            sm = self.tracked_messages.get(react.message.id)
            if sm:
                await sm.add_popup(react.message.channel)

    @commands.Cog.listener()
    async def on_reaction_remove(self, react, user):
        settings = self.bot.settings[react.message.guild]
        if settings.edit_emoji in str(react.emoji) and react.count == 0:
            sm = self.tracked_messages[react.message.id]
            await sm.delete_popup()

    @commands.Cog.listener()
    async def on_message_edit(self, before, after):
        """Adds message to the store of edits"""
        if before.author == self.bot.user:
            return

        sm = self.tracked_messages.get(before.id)
        # have to manually give datetime.now() cause discord.py is broken
        if sm and sm.add_edit(before, after, datetime.now()):
            await sm.edit_popup()
            return
        sm = smart_message(before)
        sm.add_edit(before, after, datetime.now())
        self.tracked_messages[before.id] = sm

    async def on_message_delete(self, msg):
        settings = self.bot.settings[msg.guild]
        if msg.id in self.bot.deletable_messages:
            self.bot.deletable_messages.remove(msg.id)
            return
        if msg.author != self.user and settings.repost_del_msg:
            em = discord.Embed(title=timezone_aware_format(msg.created_at), description=msg.content, colour=0x42f468)
            em.set_author(name=msg.author.display_name, icon_url=msg.author.avatar_url)
            repost = await msg.channel.send(embed=em)
            # place the tracked edits for the deleted message on the reposted embed
            if msg.id in self.tracked_messages:
                self.tracked_messages[repost.id] = self.tracked_messages[msg.id]


def setup(bot):
    bot.add_cog(EditTrackerCog(bot))
