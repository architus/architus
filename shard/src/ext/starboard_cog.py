from discord.ext import commands
import discord

from lib.config import logger
from lib.ipc import manager_pb2 as message_type


class StarboardCog(commands.Cog):

    def __init__(self, bot):
        self.bot = bot
        self.starboarded_messages = []

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        guild = react.message.guild
        settings = self.bot.settings[guild]
        if settings.starboard_emoji in str(react.emoji):
            if react.count == settings.starboard_threshold:
                await self.starboard_post(react.message, guild)

    async def starboard_post(self, message, guild):
        """Post a message in the starboard channel"""
        starboard_ch = discord.utils.get(guild.text_channels, name='starboard')
        if message.id in self.starboarded_messages or not starboard_ch or message.author == self.bot.user:
            return
        logger.info("Starboarding message: " + message.content)
        self.starboarded_messages.append(message.id)
        # save author's image so this doesn't break when the change it
        em = discord.Embed(
            timestamp=message.created_at, description=message.content, colour=0x42f468)
        img = await message.author.avatar_url.read()
        data = await self.bot.manager_client.publish_file(iter([
            message_type.File(location='avatars',
                              name=str(message.author.id),
                              file=img)]))

        em.set_author(name=f"{message.author.name}#{message.author.discriminator}", icon_url=data.url)
        em.set_footer(text='#' + message.channel.name)
        if message.embeds:
            em.set_image(url=message.embeds[0].url)
        elif message.attachments:
            em.set_image(url=message.attachments[0].url)
        await starboard_ch.send(embed=em)


def setup(bot):
    bot.add_cog(StarboardCog(bot))
