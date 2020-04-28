import discord
from discord.ext import commands
import re
from src.utils import send_message_webhook
from src.architus_emoji import ArchitusEmoji
from src.generate.emoji_list import generate
from lib.config import logger

EMOJI_DIR = 'emojis'


class emoji_manager:

    def __init__(self, bot, guild):
        self.bot = bot
        self.guild = guild
        self.emojis = []

    @property
    def guild_emojis(self):
        return [e for e in self.guild.emojis if not (e.animated or e.managed)]

    def find_emoji(self, d_id=None, a_id=None, name=None):
        '''prioritize id, then loaded name, then unloaded name'''
        emoji = next((e for e in self.emojis if e.id == a_id or e.discord_id == d_id), None)

        if emoji is not None or name is None:
            return emoji

        for e in self.emojis:
            if e.name == name and e.loaded:
                return e
            elif e.name == name and emoji is None:
                emoji = e

        return emoji

    async def cache_worst_emoji(self):
        worst_emoji = next(e for e in reversed(self.emojis) if e.loaded)
        worst_emoji_discord = self.bot.get_emoji(worst_emoji.discord_id)
        await worst_emoji_discord.delete(reason="cached")
        worst_emoji.cache()

    async def initialize(self):
        # populate emojis from db here
        dupes = await self.synchronize()
        if self.bot.settings[self.guild].manage_emojis:
            for e in dupes:
                await self.notify_deletion(e)
                await e.delete(reason="duplicate")

            if len(self.guild_emojis) >= self.max_emojis:
                await self.cache_worst_emoji()

    def sort(self):
        self.emojis.sort(key=lambda e: e.priority, reverse=True)

    async def synchronize(self):
        '''sync in memory list with any changes from real life'''

        loaded_ids = []
        matched_indices = []
        duplicate_emojis = []

        # update emojis loaded while we were offline
        for emoji in self.guild_emojis:
            loaded_ids.append(emoji.id)
            # TODO this could be optimized later to not redownload images
            a_emoji = await ArchitusEmoji.from_discord(emoji)

            try:
                i = self.emojis.index(a_emoji)
                # for if name/discord id changed
                self.emojis[i].update(a_emoji)
            except ValueError:
                self.emojis.append(a_emoji)
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

    async def notify_deletion(self, emoji: discord.Emoji):
        user = emoji.user
        if user is None:
            logger.debug("I don't know who created this emoji")
            return
        await user.send(
            f"**Notice:** your emoji has been deleted from {self.guild.name} because it is a duplicate.\n"
            "Type the emoji name as you normally would (`:{emoji.name}:`) if it was cached.\n"
            f"emoji: {emoji.url}")

    async def bump_emoji(self, emoji: ArchitusEmoji):
        '''boost an emoji's priority, while lowering all others'''
        if emoji.priority >= 100:
            return
        penalty = 0.5 / len(self.emojis)
        emoji.priority += 1

        for e in self.emojis:
            if e.priority <= -100:
                continue
            e.priority -= penalty

        self.sort()

    @property
    def max_emojis(self):
        '''get the max number of emojis the guild can hold'''
        return (50, 100, 150, 250)[self.guild.premium_tier]

    async def rename_emoji(self, before, after):
        logger.debug(f'renamed emoji {before.name}->{after.name}')
        e = next((e for e in self.emojis if before.id == e.discord_id), None)
        if e is None:
            logger.warning(f"someone renamed an emoji that I don't know about! {after.name}:{after.id}")
        else:
            e.update_from_discord(after)

    async def add_emoji(self, emoji):
        logger.debug(f"added emoji: {emoji}")
        if emoji.animated or emoji.managed:
            return
        if emoji.user != self.bot.user:
            a_emoji = await ArchitusEmoji.from_discord(emoji)

            # check if new emoji is a duplicate
            if a_emoji in self.emojis:
                logger.debug(f"duplicate emoji added!: {emoji}")
                if self.bot.settings[self.guild].manage_emojis:
                    await emoji.delete(reason="duplicate")
                    await self.notify_deletion(emoji)
            else:
                self.emojis.append(a_emoji)

            if len(self.guild_emojis) >= self.max_emojis:
                await self.cache_worst_emoji()
            self.sort()

    def list_unloaded(self):
        # return [e.name for e in self.emojis if not e.loaded] or ('No cached emojis',)
        unloaded = [e for e in self.emojis if not e.loaded]
        return generate(unloaded)
        # data, _ = await self.bot.manager_client.publish_file(data=base64.b64encode(img).decode('ascii'))
        # em = discord.Embed(title="Cached Emojis", description=ctx.guild.name)
        # em.set_image(url=data['url'])
        # em.color = 0x7b8fb7
        # if len(unloaded) == 0:
        # retur

    async def scan(self, message):
        pattern = re.compile(r'(?:<:(?P<name>\w+):(?P<id>\d+)>)|(?::(?P<nameonly>\w+):)')
        emojis = pattern.finditer(message.content)
        for emojistr in emojis:
            if emojistr['nameonly']:
                try:
                    emoji = await self.bump_emoji(self.find_emoji(name=emojistr['nameonly']))
                except Exception:
                    logger.exception('')
                    continue
                if not emoji:
                    continue
                try:
                    await send_message_webhook(
                        message.channel,
                        message.content.replace(':%s:' % emojistr['nameonly'], str(emoji)),
                        username=message.author.display_name,
                        avatar_url=str(message.author.avatar_url_as(format='png')))
                except Exception:
                    logger.exception(f"Couldn't send message with webhook")
                else:
                    self.bot.deletable_messages.append(message.id)
                    await message.delete()
                break

            elif emojistr['name']:
                emoji = self.find_emoji(d_id=emojistr['id'], a_id=emojistr['id'], name=emojistr['name'])
                if emoji:
                    await self.bump_emoji(emoji)


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
            self._managers = {guild.id: emoji_manager(self.bot, guild) for guild in self.bot.guilds}
        return self._managers

    @commands.Cog.listener()
    async def on_ready(self):
        logger.debug("initializing emojis")
        for g, m in self.managers.items():
            await m.initialize()
        logger.debug("done initializing emojis")

    @commands.command(aliases=['emotes', 'emoji', 'emote'])
    async def emojis(self, ctx):
        '''
        List currently cached emojis.
        Enclose the name (case sensitive) of cached emoji in `:`s to auto-load it into a message
        '''
        settings = self.bot.settings[ctx.guild]
        if not settings.manage_emojis:
            message = f"The emoji manager is disabled, you can enable it in `{settings.command_prefix}settings`"
        else:
            # message = '```\n • ' + '\n • '.join(self.managers[ctx.guild.id].list_unloaded()) + '```\n'
            file = self.managers[ctx.guild.id].list_unloaded()
            message = "Enclose the name (case sensitive) of cached emoji in `:`s to auto-load it into a message"

        await ctx.channel.send(message, file=discord.File(file, "cool.png"))

    @commands.Cog.listener()
    async def on_message(self, message):
        settings = self.bot.settings[message.guild]
        if settings.manage_emojis:
            await self.managers[message.guild.id].scan(message)

    @commands.Cog.listener()
    async def on_guild_emojis_update(self, guild, before, after):
        settings = self.bot.settings[guild]
        if not settings.manage_emojis:
            return

        if len(before) == len(after):  # if renamed
            diff = [i for i in range(len(after)) if before[i].name != after[i].name and not before[i].animated]
            for i in diff:
                await self.managers[guild.id].rename_emoji(before[i], after[i])

        elif len(before) > len(after):  # if removed
            for emoji in (emoji for emoji in before if emoji not in after and not emoji.animated):
                continue
                # self.managers[guild.id].del_emoji(emoji)

        elif len(after) > len(before):  # if added
            for emoji in (emoji for emoji in after if emoji not in before and not emoji.animated):
                await self.managers[guild.id].add_emoji(emoji)


def setup(bot):
    bot.add_cog(EmojiManagerCog(bot))
