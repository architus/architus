import discord
from discord.ext import commands
from PIL import Image

import re
from typing import Optional, List
from io import BytesIO
import base64

from src.utils import send_message_webhook
from src.architus_emoji import ArchitusEmoji
from src.generate.emoji_list import generate
from lib.config import logger
from lib.models import Emoji as EmojiModel

EMOJI_DIR = 'emojis'


class EmojiManager:

    def __init__(self, bot, guild: discord.guild) -> None:
        self.bot = bot
        self.session = bot.session
        self.settings = self.bot.settings[guild]
        self.guild = guild
        self.emojis = []
        self.ignore_add = []
        self.emoji_pattern = re.compile(r'(?:<:(?P<name>\w+):(?P<id>\d+)>)|(?::(?P<nameonly>\w+):)')

    def _populate_from_db(self) -> None:
        """queries the db for this guild's emojis and populates the list"""

        emojis = self.session.query(EmojiModel).filter_by(guild_id=self.guild.id).order_by(EmojiModel.priority).all()
        self.emojis = [
            ArchitusEmoji(
                Image.open(BytesIO(e.img)),
                e.name,
                e.id,
                e.discord_id,
                e.author_id,
                e.num_uses,
                e.priority)
            for e in emojis]

    def _insert_into_db(self, emoji: ArchitusEmoji) -> None:
        """stores an emoji in the database"""

        with BytesIO() as buf:
            emoji.im.save(buf, format="PNG")
            binary = buf.getvalue()

        row = EmojiModel(
            emoji.id,
            emoji.discord_id,
            emoji.author_id,
            self.guild.id,
            emoji.name,
            emoji.num_uses,
            emoji.priority,
            binary)

        try:
            self.session.add(row)
        except Exception:
            self.session.rollback()
            raise
        else:
            self.session.commit()

    def _update_emojis_db(self, emojis_list: List[ArchitusEmoji]) -> None:
        emojis = [{
            'id': e.id,
            'discord_id': e.discord_id,
            'author_id': e.author_id,
            'guild_id': self.guild.id,
            'name': e.name,
            'num_uses': e.num_uses,
            'priority': e.priority,
            # EmojiModel.id: e.id,
            # EmojiModel.discord_id: e.discord_id,
            # EmojiModel.author_id: e.author_id,
            # EmojiModel.guild_id: self.guild.id,
            # EmojiModel.name: e.name,
            # EmojiModel.num_uses: e.num_uses,
            # EmojiModel.priority: e.priority,
        } for e in emojis_list]

        try:
            self.session.bulk_update_mappings(EmojiModel, emojis)
        except Exception:
            self.session.rollback()
            raise
        else:
            self.session.commit()

    @property
    def guild_emojis(self) -> List[discord.Emoji]:
        return [e for e in self.guild.emojis if not (e.animated or e.managed)]

    def find_emoji(
            self,
            d_id: Optional[int] = None,
            a_id: Optional[int] = None,
            name: Optional[str] = None
    ) -> Optional[ArchitusEmoji]:
        """take a snowflake, hoar_frost, or name to get the ArchitusEmoji representation of an Emoji in the guild

        prioritize id, then loaded name, then unloaded name
        """

        emoji = next((e for e in self.emojis if e.id == a_id or (d_id is not None and e.discord_id == d_id)), None)

        if emoji is not None or name is None:
            return emoji

        for e in self.emojis:
            # logger.debug(f"name: {e.name} == {name}")
            if e.name == name and e.loaded:
                return e
            elif e.name == name and emoji is None:
                emoji = e

        return emoji

    async def cache_worst_emoji(self) -> None:
        """find loaded emoji with the worst priority and cache it"""
        worst_emoji = next(e for e in reversed(self.emojis) if e.loaded)
        await self.cache_emoji(worst_emoji)

    async def initialize(self) -> None:
        # populate emojis from db here
        self._populate_from_db()
        dupes = await self.synchronize()
        if self.settings.manage_emojis:
            for e in dupes:
                await self.notify_deletion(e)
                await e.delete(reason="duplicate")

            if len(self.guild_emojis) >= self.max_emojis:
                await self.cache_worst_emoji()

    def sort(self) -> None:
        """sort the list of emojis for the guild by their priority"""
        self.emojis.sort(key=lambda e: e.priority, reverse=True)

    async def synchronize(self) -> None:
        """sync in memory list with any changes from real life"""

        loaded_ids = []
        matched_indices = []
        duplicate_emojis = []

        # update emojis loaded while we were offline
        for emoji in await self.guild.fetch_emojis():
            if emoji.managed or emoji.animated:
                continue

            loaded_ids.append(emoji.id)
            # TODO this could be optimized later to not redownload images
            a_emoji = await ArchitusEmoji.from_discord(emoji)

            try:
                i = self.emojis.index(a_emoji)
                # for if name/discord id changed
                self.emojis[i].update(a_emoji)
            except ValueError:
                self.emojis.append(a_emoji)
                self._insert_into_db(a_emoji)
                i = len(self.emojis) - 1

            # check if a 'real' emoji matched more than one architus emoji
            if i in matched_indices:
                duplicate_emojis.append(emoji)
            else:
                matched_indices.append(i)

        # update emojis unloaded while we were offline
        for emoji in self.emojis:
            if emoji.discord_id not in loaded_ids:
                emoji.cache()

        self.sort()
        return duplicate_emojis

    async def notify_deletion(self, emoji: discord.Emoji) -> None:
        """send a dm to whomever's emoji we just deleted"""
        user = emoji.user
        if user is None:
            logger.debug("I don't know who created this emoji")
            return
        try:
            await user.send(
                f"**Notice:** your emoji has been deleted from {self.guild.name} because it is a duplicate.\n"
                f"Type the emoji name as you normally would (`:{emoji.name}:`) if it was cached.\n"
                f"emoji: {emoji.url}")
        except Exception:
            logger.exception('')

    async def cache_emoji(self, emoji: ArchitusEmoji) -> None:
        """remove an emoji from the guild"""
        discord_emoji = self.bot.get_emoji(emoji.discord_id)
        if discord_emoji is None:
            return

        logger.debug(f"cache {emoji} from {self.guild.name}")
        await discord_emoji.delete(reason="cached")
        emoji.cache()
        # no need to update the db here cause we're about to trigger the on_emoji_removed event

    async def load_emoji(self, emoji: ArchitusEmoji) -> ArchitusEmoji:
        if emoji.loaded:
            logger.debug(f"{emoji} already loaded")
            self.sort()
            return emoji
        while len(self.guild_emojis) >= self.max_emojis - 1:
            await self.cache_worst_emoji()
        logger.debug(f"loading {emoji}")

        self.ignore_add.append(emoji)

        with BytesIO() as buf:
            emoji.im.save(buf, format="PNG")
            binary = buf.getvalue()

        emoji.update_from_discord(
            await self.guild.create_custom_emoji(
                name=emoji.name,
                image=binary,
                reason="loaded from cache")
        )
        self.sort()
        return emoji

    async def bump_emoji(self, emoji: ArchitusEmoji) -> ArchitusEmoji:
        """boost an emoji's priority, while lowering all others"""
        if emoji.priority >= 100:
            return emoji
        logger.debug(f"bumping {emoji}")
        penalty = 0.5 / len(self.emojis)
        emoji.priority += 0.5 + penalty

        for e in self.emojis:
            if e.priority <= -100:
                continue
            e.priority -= penalty

        return await self.load_emoji(emoji)

    @property
    def max_emojis(self) -> int:
        """get the max number of emojis the guild can hold"""
        return (50, 100, 150, 250)[self.guild.premium_tier]

    async def on_emoji_renamed(self, before, after) -> None:
        """updates our version of an emoji that just got renamed"""
        logger.debug(f'renamed emoji {before.name}->{after.name}')
        e = next((e for e in self.emojis if before.id == e.discord_id), None)
        if e is None:
            logger.warning(f"someone renamed an emoji that I don't know about! {after.name}:{after.id}")
        else:
            e.update_from_discord(after)
            self._update_emojis_db((e,))

    async def add_emoji(self, emoji: ArchitusEmoji) -> None:
        """Inserts an emoji into the guild, making space if necessary
        does not check for duplicates
        """
        logger.debug(f"added emoji: {emoji}")
        while len(self.guild_emojis) >= self.max_emojis:
            await self.cache_worst_emoji()

        self.emojis.append(emoji)
        self._insert_into_db(emoji)
        self.sort()

    async def on_emoji_removed(self, emoji: discord.Emoji) -> None:
        """flags an emoji as cached
        should be called when an emoji is removed from the guild
        """
        emoji = self.find_emoji(d_id=emoji.id, name=emoji.name)
        if emoji:
            emoji.cache()
            self._update_emojis_db((emoji,))

    async def on_react(self, react: discord.Reaction) -> None:
        emoji = self.find_emoji(d_id=react.emoji.id, name=react.emoji.name)
        if emoji:
            await self.bump_emoji(emoji)

    async def on_emoji_added(self, emoji: discord.Emoji) -> None:
        """checks if the new emoji is a duplicate and adds it if not. also fetches the uploader
        should be called when an emoji is removed from the guild
        """
        if emoji.animated or emoji.managed:
            return

        a_emoji = await ArchitusEmoji.from_discord(emoji)

        if a_emoji in self.ignore_add:
            self.ignore_add.remove(a_emoji)
            logger.debug(f"ignoring {a_emoji}")
            return

        # check if new emoji is a duplicate
        if a_emoji in self.emojis:
            logger.debug(f"duplicate emoji added!: {emoji}")
            if self.settins.manage_emojis:
                await emoji.delete(reason="duplicate")
                await self.notify_deletion(emoji)
        else:
            # get the user
            emoji = await self.guild.fetch_emoji(emoji.id)
            await self.add_emoji(a_emoji.update_from_discord(emoji))

    def list_unloaded(self) -> Optional[BytesIO]:
        """generates an image preview list of the unloaded emojis"""
        # return [e.name for e in self.emojis if not e.loaded] or ('No cached emojis',)
        unloaded = [e for e in self.emojis if not e.loaded]
        if len(unloaded) == 0:
            return None
        return generate(unloaded)

    async def scan(self, msg):
        """scans a message for cached emoji and, if it finds any, loads the emoji and replaces the message"""
        if msg.author.bot:
            return
        content = msg.content
        matches = self.emoji_pattern.finditer(content)
        replace = False
        did_match = False

        for match in matches:
            did_match = True
            if match['nameonly']:
                emoji = self.find_emoji(name=match['nameonly'])
                if emoji is not None:
                    emoji.num_uses += 1
                    replace = replace if emoji.loaded else True
                    await self.bump_emoji(emoji)
                    content = content.replace(f":{emoji.name}:", emoji.to_discord_str())
            else:
                emoji = self.find_emoji(d_id=match['id'], a_id=match['id'], name=match['name'])
                if emoji is not None:
                    emoji.num_uses += 1
                    replace = replace if emoji.loaded else True
                    await self.bump_emoji(emoji)
                    content = content.replace(f"<:{emoji.name}:{match['id']}>", emoji.to_discord_str())
        if did_match:
            self._update_emojis_db(self.emojis)
        if replace and self.settings.manage_emojis:
            try:
                await send_message_webhook(
                    msg.channel,
                    content,
                    username=msg.author.display_name,
                    avatar_url=str(msg.author.avatar_url_as(format='png')))
            except Exception:
                logger.exception(f"Couldn't send message with webhook")
            else:
                self.bot.deletable_messages.append(msg.id)
                await msg.delete()


class EmojiManagerCog(commands.Cog, name="Emoji Manager"):
    '''
    Can be used to hotswap extra emojis into the server when the limit is reached
    must be enabled in settings
    '''

    def __init__(self, bot):
        self.bot = bot
        self._managers = None

    @property
    def managers(self):
        if self._managers is None:
            self._managers = {guild.id: EmojiManager(self.bot, guild) for guild in self.bot.guilds}
        return self._managers

    @commands.Cog.listener()
    async def on_ready(self):
        logger.debug("initializing emoji managers...")
        for _, m in self.managers.items():
            await m.initialize()
        logger.debug("emoji managers ready")

    @commands.command(aliases=['emotes', 'emoji', 'emote'])
    async def emojis(self, ctx):
        """
        List currently cached emojis.
        Enclose the name (case sensitive) of cached emoji in `:`s to auto-load it into a message
        """
        settings = self.bot.settings[ctx.guild]
        if not settings.manage_emojis:
            message = f"The emoji manager is disabled, you can enable it in `{settings.command_prefix}settings`"
        else:
            # message = '```\n • ' + '\n • '.join(self.managers[ctx.guild.id].list_unloaded()) + '```\n'
            file = self.managers[ctx.guild.id].list_unloaded()
            if file:
                message = "Enclose the name (case sensitive) of cached emoji in `:`s to auto-load it into a message"
                # msg = await ctx.send(message, file=discord.File(file, "cool.png"))
                data, _ = await self.bot.manager_client.publish_file(
                    data=base64.b64encode(file.getvalue()).decode('ascii'))
                em = discord.Embed(title="Cached Emojis", description=ctx.guild.name)
                # em.set_image(url=msg.attachments[0].url)
                em.set_image(url=data['url'])
                em.color = 0x7b8fb7
                em.set_footer(text=message)
                await ctx.send(embed=em)
            else:
                await ctx.send("No cached emoji")

    @commands.command(aliases=['emoji_ranks', 'emoji_elo'], hidden=True)
    async def emojilo(self, ctx):
        """display the elo of each emoji in the guild"""
        settings = self.bot.settings[ctx.guild]
        if settings.bot_commands_channels and ctx.channel.id not in settings.bot_commands_channels:
            await ctx.send(f"Please use <#{settings.bot_commands_channels[0]}>")
            return
        manager = self.managers[ctx.guild.id]
        message = '```\n' + "\n".join([f" • {e.priority:5.2f} : {e.name}" for e in manager.emojis]) + '```\n'
        await ctx.send(message[:1997] + '```')
        if len(message > 2000):
            await ctx.send('```' + message[1997:])

    @commands.Cog.listener()
    async def on_message(self, msg):
        await self.managers[msg.guild.id].scan(msg)

    @commands.Cog.listener()
    async def on_reaction_add(self, react, user):
        await self.managers[react.message.channel.guild.id].on_react(react)

    @commands.Cog.listener()
    async def on_guild_emojis_update(self, guild, before, after):
        if len(before) == len(after):  # if renamed
            diff = [i for i in range(len(after)) if before[i].name != after[i].name and not before[i].animated]
            for i in diff:
                await self.managers[guild.id].on_emoji_renamed(before[i], after[i])

        elif len(before) > len(after):  # if removed
            for emoji in (emoji for emoji in before if emoji not in after and not emoji.animated):
                await self.managers[guild.id].on_emoji_removed(emoji)

        elif len(after) > len(before):  # if added
            for emoji in (emoji for emoji in after if emoji not in before and not emoji.animated):
                await self.managers[guild.id].on_emoji_added(emoji)


def setup(bot):
    bot.add_cog(EmojiManagerCog(bot))
