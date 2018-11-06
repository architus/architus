import discord
import os
import aiofiles, aiohttp

EMOJI_DIR = 'emojis'
MAX_ENABLED = 49

class emoji_manager():
 
    def __init__(self, server):
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
            return 0

    def _get_highest_available_priority(self):
        priorities = [int(x[0]) for x in self.saved_emojis]
        maxp = max(priorities)
        for i in range(1,maxp):
            if i not in priorities: return i
        return maxp + 1
            
    def _path(self, emoji, priority=None):
        if not priority:
            priority = self._get_priority(emoji)
        return self.dir + '/' + str(priority) + '-' + emoji.name + '-' + emoji.id

    def _get_by_priority(self, priority):
        '''returns emoji from self.server'''
        return discord.utils.get(self.server.emojis, id=next(x for x in self.saved_emojis if x[2] == emoji.id)[2])

    async def remove_emoji(self, emoji):
        try:
            print('removing %s\'s %s' % (self.server.name, emoji.name))
            self.dirty = True
            os.remove(self._path(emoji))
        except: pass

    async def bump_emoji(self, emoji):
        priority = self._get_priority(emoji)
        self.rename_emoji(emoji, emoji, after_priority=(priority - (1 if priority > 1 else 0)))
        down_bump_emoji = self._get_by_priority(priority - (1 if priority > 1 else 0))
        self.rename_emoji(down_bump_emoji, down_bump_emoji, after_priority=priority)

    async def rename_emoji(self, before, after, after_priority=None):
        try:
            print('renaming %s\'s %s to %s' % (self.server.name, before.name, after.name))
            self.dirty = True
            os.rename(self._path(before), self._path(after, priority=after_priority))
        except: pass

    @property
    def saved_emojis(self):
        if self.dirty:
            try:
                print ('reloading array')
                emojis = [x.split('-') for x in os.listdir(EMOJI_DIR + '/' + self.server.id)]
                self.dirty = False
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
        print (self.saved_emojis)
