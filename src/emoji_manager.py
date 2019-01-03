import discord
import os, re, time
import aiofiles, aiohttp, asyncio
import requests
from src.webhook import send_message

EMOJI_DIR = 'emojis'
MAX_ENABLED = 49

def is_animated(emoji):
    return False
    animated = requests.get(emoji.url[:-3] + 'gif').status_code == 200
    if animated: print(emoji.name + " is animated")
    return animated

class emoji_manager():
    def __init__(self, client, server, deletable_messages):
        self.client = client
        self.server = server
        self.deletable_messages = deletable_messages
        self._priorities = self.server.emojis.copy()
        try:
            os.makedirs(EMOJI_DIR + '/' + self.server.id)
        except: pass

    def list_unloaded(self):
        return os.listdir(EMOJI_DIR + '/' + self.server.id) or ['No cached emojis']

    async def clean(self):
        '''renames any emoji on the server that shares a name with an emoji on the disk or on the server'''
        print("renaming dupes")
        names = os.listdir(EMOJI_DIR + '/' + self.server.id)
        for emoji in self.server.emojis:
            if is_animated(emoji): continue
            count = 1
            name = emoji.name
            if name in names:
                while name + str(count) in names:
                    count += 1
                print("renaming %s to %s" % (emoji.name, emoji.name + str(count)))
                await self.client.edit_custom_emoji(emoji, name=emoji.name + str(count))
            names.append(emoji.name)
        if len(self.server.emojis) > MAX_ENABLED and False:
            print("caching one emoji...")
            await self.client.delete_custom_emoji(self.server.emojis[-1])


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
                await self.client.delete_message(message)
                send_message(message.channel,
                        message.content.replace(':%s:' % emojistr.group('nameonly'), str(emoji)),
                        username=message.author.display_name,
                        avatar_url=message.author.avatar_url,
                        embeds=message.embeds
                )
                break

            elif emojistr.group('name'):
                emoji = discord.utils.get(self.server.emojis, id=emojistr.group('id'), name=emojistr.group('name'))
                if emoji: await self.bump_emoji(emoji)

    async def rename_emoji(self, before, after):
        print('renamed')
        await self.clean()

    async def add_emoji(self, emoji):
        '''call this when an emoji is added to the server'''
        await self.clean()
        print('added ' + str(emoji))
        if len(self.server.emojis) > MAX_ENABLED:
            await self._save(self._priorities[-1])
            await self.client.delete_custom_emoji(self._priorities[-1])
        self._priorities.append(emoji)

    async def bump_emoji(self, emoji):
        '''call this when an emoji is used or requested'''
        print('bumped ' + str(emoji))
        if emoji in self.server.emojis:
            i = self._priorities.index(emoji)
            if i != 0:
                self._priorities[i] = self._priorities[i-1]
                self._priorities[i-1] = emoji
        else:
            image = await self._load(emoji)
            return await self.client.create_custom_emoji(self.server, name=emoji, image=image)

    def del_emoji(self, emoji):
        print('deleted ' + str(emoji))
        '''call this when an emoji is deleted (even if by the manager)'''
        self._priorities.remove(emoji)

    def _path(self, emoji):
        try:
            return "%s/%s/%s" % (EMOJI_DIR, self.server.id, emoji.name)
        except:
            return "%s/%s/%s" % (EMOJI_DIR, self.server.id, emoji)

    async def _load(self, emoji):
        print('loaded ' + str(emoji))
        f = await aiofiles.open(self._path(emoji), 'rb')
        binary = await f.read()
        await f.close()
        os.remove(self._path(emoji))
        return binary

    async def _save(self, emoji):
        print('saving ' + str(emoji))
        '''load all emojis in the server into memory'''
        async with aiohttp.ClientSession() as session:
            async with session.get(emoji.url) as resp:
                if resp.status == 200:
                    f = await aiofiles.open(self._path(emoji), mode='wb')
                    await f.write(await resp.read())
                    await f.close()
