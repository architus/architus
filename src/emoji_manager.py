import discord
from discord.ext import commands
import os, re, time
import aiofiles, aiohttp, asyncio
import requests
from src.webhook import send_message

EMOJI_DIR = 'emojis'

class emoji_manager():
    def __init__(self, client, guild, deletable_messages):
        self.client = client
        self.guild = guild
        self.deletable_messages = deletable_messages
        self._priorities = list(self.guild.emojis)
        try:
            os.makedirs(EMOJI_DIR + '/' + str(self.guild.id))
        except: pass

    @property
    def max_emojis(self):
        maxes = (49, 99, 149, 249)
        return maxes[self.guild.premium_tier]

    def list_unloaded(self):
        loaded_names = [emoji.name for emoji in self.guild.emojis]
        return [name for name in os.listdir(f"{EMOJI_DIR}/{self.guild.id}") if name not in loaded_names] or ['No cached emojis']

    async def clean(self):
        '''renames any emoji on the server that shares a name with an emoji on the disk or on the server'''
        return
        print("renaming dupes")
        names = os.listdir(f"{EMOJI_DIR}/{self.guild.id}")
        for emoji in self.guild.emojis:
            if emoji.animated: continue
            count = 1
            name = emoji.name
            if name in names:
                while name + str(count) in names:
                    count += 1
                print("renaming %s to %s" % (emoji.name, emoji.name + str(count)))
                await emoji.edit(name=emoji.name + str(count))
            names.append(emoji.name)
        if len(self.guild.emojis) > self.max_emojis and False:
            print("caching one emoji...")
            await self.guild.emojis[-1].delete(reason="cached")


    async def scan(self, message):
        pattern = re.compile('(?:<:(?P<name>\w+):(?P<id>\d+)>)|(?::(?P<nameonly>\w+):)')
        emojis = pattern.finditer(message.content)
        for emojistr in emojis:
            if emojistr.group('nameonly'):
                try:
                    emoji = await self.bump_emoji(emojistr.group('nameonly'))
                except Exception as e:
                    print(e)
                    continue
                if not emoji: continue
                self.deletable_messages.append(message.id)
                await message.delete()
                send_message(message.channel,
                        message.content.replace(':%s:' % emojistr.group('nameonly'), str(emoji)),
                        username=message.author.display_name,
                        avatar_url=str(message.author.avatar_url).replace('webp','png')
                        )
                print(str(message.author.avatar_url))
                break

            elif emojistr.group('name'):
                emoji = discord.utils.get(self.guild.emojis, id=emojistr.group('id'), name=emojistr.group('name'))
                if emoji: await self.bump_emoji(emoji)

    async def rename_emoji(self, before, after):
        print('renamed')
        await self.clean()

    async def add_emoji(self, emoji):
        '''call this when an emoji is added to the server'''
        await self.clean()
        print('added ' + str(emoji))
        if emoji in self._priorities:
            # this can happen if the emoji manager is instantiated after the emoji is added
            self._priorities.remove(emoji)
        if len(self.guild.emojis) > self.max_emojis:
            await self._save(self._priorities[-1])
            await self._priorities[-1].delete(reason="cached")
        print("inserting")
        self._priorities.insert(0, emoji)

    async def bump_emoji(self, emoji):
        '''call this when an emoji is used or requested'''
        print('bumped ' + str(emoji))
        if emoji in self.guild.emojis:
            i = self._priorities.index(emoji)
            if i != 0:
                self._priorities[i] = self._priorities[i-1]
                self._priorities[i-1] = emoji
        else:
            image = await self._load(emoji)
            return await self.guild.create_custom_emoji(name=emoji, image=image)

    def del_emoji(self, emoji):
        print('deleted ' + str(emoji))
        '''call this when an emoji is deleted (even if by the manager)'''
        self._priorities.remove(emoji)

    def _path(self, emoji):
        try:
            return "%s/%s/%s" % (EMOJI_DIR, self.guild.id, emoji.name)
        except:
            return "%s/%s/%s" % (EMOJI_DIR, self.guild.id, emoji)

    async def _load(self, emoji):
        print('loaded ' + str(emoji))
        f = await aiofiles.open(self._path(emoji), 'rb')
        binary = await f.read()
        await f.close()
        #os.remove(self._path(emoji))
        return binary

    async def _save(self, emoji):
        print('saving ' + str(emoji) + ' from ' + str(emoji.url))
        '''load all emojis in the server into memory'''
        async with aiohttp.ClientSession() as session:
            async with session.get(str(emoji.url)) as resp:
                if resp.status == 200:
                    f = await aiofiles.open(self._path(emoji), mode='wb')
                    await f.write(await resp.read())
                    await f.close()
                else:
                    print("API gave unexpected response (%d) emoji not saved" % resp.status)

class EmojiManagerCog(commands.Cog):

    def __init__(self, bot):
        self.bot = bot
        self._managers = None

    @property
    def managers(self):
        self._managers = self._managers or {guild.id : emoji_manager(self.bot, guild, []) for guild in self.bot.guilds}
        return self._managers

    @property
    def guild_settings(self):
        return self.bot.get_cog('GuildSettings')

    @commands.command(aliases=['emotes', 'emoji', 'emote'])
    async def emojis(self, ctx):
        '''List currently cached emojis. Enclose the name (case sensitive) of cached emoji in `:`s to auto-load it into a message'''
        settings = self.guild_settings.get_guild(ctx.guild)
        if not settings.manage_emojis:
            message = "The emoji manager is disabled, you can enable it in `!settings`"
        else: 
            message = '```\n • ' + '\n • '.join(self.managers[ctx.guild.id].list_unloaded()) + '```\n'
            message += "Enclose the name (case sensitive) of cached emoji in `:`s to auto-load it into a message"

        await ctx.channel.send(message)


    @commands.Cog.listener()
    async def on_message(self, message):
        settings = self.guild_settings.get_guild(message.guild)
        if settings.manage_emojis: await self.managers[message.guild.id].scan(message)


    @commands.Cog.listener()
    async def on_guild_emojis_update(self, guild, before, after):
        settings = self.guild_settings.get_guild(guild)
        if not settings.manage_emojis:
            return

        if len(before) == len(after): # if renamed
            diff = [i for i in range(len(after)) if before[i].name != after[i].name and not before[i].animated]
            for i in diff:
                await self.managers[guild.id].rename_emoji(before[i], after[i])

        elif len(before) > len(after): # if removed
            for emoji in (emoji for emoji in before if emoji not in after and not emoji.animated):
                self.managers[guild.id].del_emoji(emoji)

        elif len(after) > len(before): # if added
            for emoji in (emoji for emoji in after if emoji not in before and not emoji.animated):
                await self.managers[guild.id].add_emoji(emoji)
        

def setup(bot):
    bot.add_cog(EmojiManagerCog(bot))
