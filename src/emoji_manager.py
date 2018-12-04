import discord
import os, re, time
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

    async def scan(self, message):
        pattern = re.compile('(?:<:(?P<name>\w+):(?P<id>\d+)>)|(?::(?P<nameonly>\w+):)')
        emojis = pattern.finditer(message.content)
        for emojistr in emojis:
            if emojistr.group('nameonly'):
                emoji = self.get_by_name(emojistr.group('nameonly'))
                if emoji:
                    print('bumping %s\'s %s-%s to top 49' % (self.server, emoji.name, emoji.id))
                    self.bump_until_enabled(emoji)
                    await self._set_enabled_emojis()
            elif emojistr.group('name'):
                emoji = discord.utils.get(self.server.emojis, id=emojistr.group('id'), name=emojistr.group('name'))
                print('bumping %s\'s %s-%s' % (self.server, emoji.name, emoji.id))
                self.bump_emoji(emoji)


        print(message.content)

    def _init_dir(self):
        try:
            os.makedirs(self.dir)
        except: pass

    def _get_priority(self, emoji, perfect=False):
        try:
            return int(next(x for x in self.saved_emojis if x[2] == emoji.id)[0])
        except:
            if not perfect:
                print("couldn't find priority!")
                return self._get_highest_available_priority()
            time.sleep(0.2)
            return self._get_priority(emoji)

    def _get_highest_available_priority(self):
        priorities = [int(x[0]) for x in self.saved_emojis]
        maxp = max(priorities or [1])
        for i in range(1, maxp + 1):
            if i not in priorities: return i
        return maxp + 1

    async def _set_enabled_emojis(self):
        for emoji in self.server.emojis:
            if self._get_priority(emoji) > MAX_ENABLED:
                print ('disabling %s\'s %s' % (self.server.name, emoji.name))
                await self.client.delete_custom_emoji(emoji)
        for i in range(1, len(self.saved_emojis)):
            if len(self.server.emojis) >= MAX_ENABLED: break
            if not self.is_loaded(priority=i):
                try:
                    emoji_info = next(x for x in self.saved_emojis if int(x[0]) == i)
                except StopIteration:
                    continue
                new_emoji = fake_emoji(emoji_info[0], emoji_info[1], emoji_info[2])

                print ('enabling %s\'s %s %s' % (self.server.name, new_emoji.name, new_emoji.priority))
                with open(self._path(new_emoji, priority=new_emoji.priority), 'rb') as f:
                    updated_new_emoji = await self.client.create_custom_emoji(self.server, name=emoji_info[1], image=bytearray(f.read()))
                print(type(new_emoji))
                print(type(updated_new_emoji))
                self.rename_emoji(new_emoji, updated_new_emoji, before_priority=new_emoji.priority, after_priority=new_emoji.priority)
                print('end of set enabled')

    def _path(self, emoji, priority=None):
        if not priority:
            print('trying to get priority')
            priority = self._get_priority(emoji)
        else:
            print("got priority " + str(priority))
        return self.dir + '/' + str(priority) + '-' + emoji.name + '-' + emoji.id

    def get_by_name(self, name):
        try:
            emoji = discord.utils.get(self.server.emojis, id=next(x for x in self.saved_emojis if x[1] == name)[2])
            if not emoji:
                emoji_info = next(x for x in self.saved_emojis if x[1] == name)
                emoji = fake_emoji(emoji_info[0], emoji_info[1], emoji_info[2])
            return emoji
        except StopIteration as e: print(e);return None

    def is_loaded(self, priority=None, name=None, id=None):
        try:
            if priority:
                return bool(discord.utils.get(self.server.emojis, id=next(x for x in self.saved_emojis if int(x[0]) == priority)[2]))
            elif name:
                return bool(discord.utils.get(self.server.emojis, id=next(x for x in self.saved_emojis if x[1] == name)[2]))
            elif id:
                return bool(discord.utils.get(self.server.emojis, id=id))
        except StopIteration: return False



    def _get_by_priority(self, priority):
        '''returns emoji from self.server'''
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
        priority = self._get_priority(emoji, perfect=True)
        if priority == 1: return
        new_priority = priority - 1
        down_bump_emoji = self._get_by_priority(new_priority)
        self.rename_emoji(emoji, emoji, before_priority=priority, after_priority=new_priority)
        self.rename_emoji(down_bump_emoji, down_bump_emoji, before_priority=new_priority, after_priority=priority)

    def bump_until_enabled(self, emoji):
        priority = self._get_priority(emoji)
        if priority <= MAX_ENABLED: return
        for _ in range(priority - MAX_ENABLED):
            self.bump_emoji(emoji)

    def rename_emoji(self, before, after,before_priority=None, after_priority=None):
        try:
            print('renaming %s\'s %s to %s' % (self.server.name, before.name, after.name))
            self.dirty = True
            os.rename(self._path(before, priority=before_priority), self._path(after, priority=after_priority))
            time.sleep(.2)
        except: pass

    @property
    def saved_emojis(self):
        if self.dirty or True:
            try:
                #print ('reloading emoji array')
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
