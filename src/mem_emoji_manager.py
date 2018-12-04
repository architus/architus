import discord
import os, re, time
import aiofiles, aiohttp, asyncio

EMOJI_DIR = 'emojis'
MAX_ENABLED = 49

class emoji_manager():
    def __init__(self, client, server):
        self.client = client
        self.server = server
        self._emojis = []

    async def scan(self, message):
        pattern = re.compile('(?:<:(?P<name>\w+):(?P<id>\d+)>)|(?::(?P<nameonly>\w+):)')
        emojis = pattern.finditer(message.content)
        for emojistr in emojis:
            if emojistr.group('nameonly'):
                try:
                    emoji = [e for e in self._emojis if e.name == emojistr.group('nameonly')][0]
                    print('inserting %s\'s %s-%s' % (self.server, emoji.name, emoji.id))
                    await self.insert_emoji(emoji)
                except IndexError as e:
                    print(e)
                    pass
            elif emojistr.group('name'):
                emoji = discord.utils.get(self.server.emojis, id=emojistr.group('id'), name=emojistr.group('name'))
                print('bumping %s\'s %s-%s' % (self.server, emoji.name, emoji.id))
                #self.bump_emoji(emoji)

    def _path(self, emoji):
        return "%s/%s/%s-%s-%s" % (EMOJI_DIR, self.server.id, emoji.priority,
                emoji.name, emoji.id)

    def _bump(self, emoji):
        '''increase the priority of an emoji in the list'''
        i = self._emojis.index(emoji)
        priority = self._emojis[i].priority
        try:
            old_emoji = [e for e in self._emojis if e.priority == priority - 1][0]
            old_emoji.priority += 1
        except:
            print("couldn't find emoji of priority " + str(int(priority) - 1))
        emoji.priority -= 1 if priority is not 1 else 0
        print("bumped %s to %d" % (emoji.name, emoji.priority))

    def rename_emoji(self, before, after):
        self._emojis[self._emojis.index(before)].name = after.name
        self.print_emojis()

    async def save_emojis_disk(self):
        '''save any emoji that's been modified to disk'''
        emojis = [x.split('-') for x in os.listdir(EMOJI_DIR + '/' + self.server.id)]
        emoji_ids = [x[2] for x in emojis]
        for emoji in self._emojis:
            if emoji.changed or emoji.id not in emoji_ids:
                if emoji.id in emoji_ids:
                    i = emoji_ids.index(emoji.id)
                    os.remove(self._path(smart_emoji(
                        id=emojis[i][2], name=emojis[i][1], priority=emojis[i][0])))
                f = await aiofiles.open(self._path(emoji), mode='wb')
                await f.write(emoji.binary)
                await f.close()

    async def _load_binary_disk(self, priority, name, id):
        '''get the binary image data from a file'''
        f = await aiofiles.open("%s/%s/%s-%s-%s" % (EMOJI_DIR, self.server.id, priority, name, id), 'rb')
        binary = await f.read()
        await f.close()
        return binary

    def _loaded(self, emoji):
        return bool(discord.utils.get(self.server.emojis, id=emoji.id, name=emoji.name))

    async def load_emoji_disk(self, name=None, id=None):
        '''create a fake emoji(s) from file. returns all emojis if name is None'''
        emojis = [x.split('-') for x in os.listdir(EMOJI_DIR + '/' + self.server.id)]
        if name:
            emojis = [x for x in emojis if x[1] == name]
            if id: emojis = [x for x in emojis if x[2] == id]
        return [smart_emoji(id=x[2], priority=x[0], name=x[1],
            binary=await self._load_binary_disk(x[0], x[1], x[2])) for x in emojis]

    async def insert_emoji(self, emoji):
        '''load an emoji into the discord server'''
        while int(emoji.priority) > MAX_ENABLED:
            self._bump(emoji)
        if len(self.server.emojis) >= MAX_ENABLED:
            for e in self._emojis:
                print(e)
                if e.priority == MAX_ENABLED + 1:
                    try:
                        ded_emoji = [x for x in self.server.emojis if e == x][0]
                        await self.client.delete_custom_emoji(ded_emoji)
                    except Exception as ex:
                        print(ex)
                        print("we thought an emoji was loaded but it wasn't...")
                        continue
                    print('deleting ' + str(_path(e)))
                    os.remove(self._path(e)) # delete manually cause we duped this emoji
                    self._emojis.remove(e)
                    emoji.id = (await self.client.create_custom_emoji(self.server, name=emoji.name, image=emoji.binary)).id
        await self.save_emojis_disk()

    async def build_emojis_list(self):
        self._emojis = []
        await self._download_emojis()
        disk_emojis = await self.load_emoji_disk()
        self._emojis.extend([e for e in disk_emojis if not self._loaded(e)])
        self.print_emojis()


    async def add_emoji(self):
        await self.build_emojis_list()
        try:
            e = [x for x in self._emojis if x.priority == MAX_ENABLED + 1][0]
            self._bump(e)
            e = [x for x in self._emojis if x.priority == MAX_ENABLED + 1][0]
            try:
                ded_emoji = [x for x in self.server.emojis if e == x][0]
                await self.client.delete_custom_emoji(ded_emoji)
            except Exception as e:
                print(e)
                print("we thought an emoji was loaded but it wasn't (1)...")
        except IndexError as e:
            print(e)
            print('not full')

        #self._emojis.append(smart_emoji(emoji=emoji, priority=max([e.priority for e in self._emojis])+1, binary

    async def _download_emojis(self):
        '''load all emojis in the server into memory'''
        count = 0
        for emoji in self.server.emojis:
            count += 1
            async with aiohttp.ClientSession() as session:
                async with session.get(emoji.url) as resp:
                    if resp.status == 200:
                        self._emojis.append(smart_emoji(emoji=emoji, priority=count, binary=await resp.read()))

    def print_emojis(self):
        for emoji in self._emojis:
            print("%s-%s-%s %sb" % (emoji.priority, emoji.name, emoji.id, len(emoji.binary)))

class smart_emoji:
    def __init__(self, emoji=None, id=None, priority=None, name=None, binary=None):
        if emoji and priority:
            self._name = emoji.name
            self._id = emoji.id
        elif id and priority and name:
            self._id = id
            self._name = name
        else:
            raise ValueError("improper args")
        self._priority = int(priority)
        self._binary = binary
        self.changed = False

    def __eq__(self, emoji):
        return emoji.id == self.id and emoji.name == self.name

    @property
    def name(self) -> str:
        return self._name

    @name.setter
    def name(self, name: str):
        self._name = name
        self.changed = True

    @property
    def id(self) -> str:
        return self._id

    @id.setter
    def id(self, id: str):
        self._id = id
        self.changed = True

    @property
    def priority(self) -> int:
        return self._priority

    @priority.setter
    def priority(self, priority):
        self._priority = int(priority)
        self.changed = True

    @property
    def binary(self) -> str:
        return self._binary

    @binary.setter
    def binary(self, binary: str):
        self._binary = binary
        self.changed = True
