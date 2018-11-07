import discord
import os
import aiofiles, aiohttp, asyncio

EMOJI_DIR = 'emojis'
MAX_ENABLED = 49

class emoji_manager():
 
    def __init__(self, client, server):
        self.client = client
        self.server = server
        self.dir = EMOJI_DIR + '/' + self.server.id
        self._init_dir()
        self._saved_emojis = []
        self.dirty = True

    def _init_dir(self):
        try:
            os.makedirs(self.dir)
        except: pass

    def _get_priority(self, emoji):
        try:
            return int(next(x for x in self.saved_emojis if x[2] == emoji.id)[0])
        except:
            return self._get_highest_available_priority()

    def _get_highest_available_priority(self):
        priorities = [int(x[0]) for x in self.saved_emojis]
        maxp = max(priorities or [1])
        for i in range(1, maxp + 1):
            if i not in priorities: return i
        return maxp + 1

    async def _set_enabled_emojis(self):
        for i in range(1, len(self.saved_emojis)):
            if len(self.server.emojis) >= MAX_ENABLED: break
            if not self._get_by_priority(i):
                try:
                    emoji_info = next(x for x in self.saved_emojis if int(x[0]) == i)
                except StopIteration:
                    continue
                new_emoji = fake_emoji(emoji_info[0], emoji_info[1], emoji_info[2])

                print ('enabling %s\'s %s' % (self.server.name, new_emoji.name))
                with open(self._path(new_emoji, priority=new_emoji.priority), 'rb') as f:
                    updated_new_emoji = await self.client.create_custom_emoji(self.server, name=emoji_info[1], image=bytearray(f.read()))
                await self.rename_emoji(new_emoji, updated_new_emoji)
        for emoji in self.server.emojis:
            print('emoji: %s-%s, priority %s' % (emoji.name,emoji.id,self._get_priority(emoji)))
            if self._get_priority(emoji) > MAX_ENABLED:
                print ('disabling %s\'s %s' % (self.server.name, emoji.name))
                await self.client.delete_custom_emoji(emoji)
 
    def _path(self, emoji, priority=None):
        if not priority:
            priority = self._get_priority(emoji)
        return self.dir + '/' + str(priority) + '-' + emoji.name + '-' + emoji.id

    def _get_by_priority(self, priority):
        '''returns emoji from self.server'''
        print('getting emoji for priority ' + str(priority))
        try:
            emoji = discord.utils.get(self.server.emojis, id=next(x for x in self.saved_emojis if int(x[0]) == priority)[2])
            if not emoji:
                emoji_info = next(x for x in self.saved_emojis if int(x[0]) == priority)
                emoji = fake_emoji(emoji_info[0], emoji_info[1], emoji_info[2])
            return emoji
        except StopIteration as e: print(e);return None

    def remove_emoji(self, emoji):
        try:
            return
            print('removing %s\'s %s' % (self.server.name, emoji.name))
            self.dirty = True
            os.remove(self._path(emoji))
        except: pass

    def bump_emoji(self, emoji):
        print('bumping %s\'s %s' % (self.server.name, emoji.name))
        priority = self._get_priority(emoji)
        if priority == 1: return
        new_priority = priority - 1
        down_bump_emoji = self._get_by_priority(new_priority)
        self.rename_emoji(emoji, emoji, after_priority=new_priority)
        self.rename_emoji(down_bump_emoji, down_bump_emoji, before_priority=new_priority, after_priority=priority)

    def bump_until_enabled(self, emoji):
        while self._get_priority(emoji) > MAX_ENABLED:
            self.bump_emoji(emoji)

    def rename_emoji(self, before, after,before_priority=None, after_priority=None):
        try:
            print('renaming %s\'s %s to %s' % (self.server.name, before.name, after.name))
            self.dirty = True
            os.rename(self._path(before, priority=before_priority), self._path(after, priority=after_priority))
        except: pass

    @property
    def saved_emojis(self):
        if self.dirty or True:
            try:
                print ('reloading emoji array')
                self.dirty = False
                emojis = [x.split('-') for x in os.listdir(EMOJI_DIR + '/' + self.server.id)]
            except: return []
            self._saved_emojis = emojis
        return self._saved_emojis

    async def save_emojis(self):
        for emoji in self.server.emojis:
            if not os.path.exists(self._path(emoji)):
                self.dirty = True
                async with aiohttp.ClientSession() as session:
                    async with session.get(emoji.url) as resp:
                        if resp.status == 200:
                            print ('saving %s\'s %s' % (self.server.name, emoji.name))
                            f = await aiofiles.open(self._path(emoji, priority=self._get_highest_available_priority()), mode='wb')
                            await f.write(await resp.read())
                            await f.close()
                self.bump_until_enabled(emoji)
        await self._set_enabled_emojis()
        print (self.saved_emojis)


class fake_emoji:
    def __init__(self, priority, name, eid):
        self.priority = priority
        self.name = name
        self.id = eid
