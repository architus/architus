class LogActions:
    MESSAGE_SEND = 3001
    MESSAGE_EDIT = 3002
    REACTION_ADD = 3100
    REACTION_REMOVE = 3101
    MESSAGE_DELETE = 72


class MockMember(object):
    def __init__(self, id=0):
        self.id = id
        self.mention = "<@%_CLIENT_ID_%>"
        self.display_name = "bad guy"
        self.bot = False


class MockRole(object):
    pass


class MockChannel(object):
    def __init__(self, bot, sends, reactions, resp_id):
        self.bot = bot
        self.sends = sends
        self.reactions = reactions
        self.resp_id = resp_id

    async def send(self, *args):
        for thing in args:
            self.sends.append(thing)
        return MockMessage(self.bot, self.resp_id, self.sends, self.reactions, 0)


class MockGuild:
    def __init__(self, id):
        self.region = 'us-east'
        self.id = int(id)
        self.owner = MockMember()
        self.me = MockMember()
        self.default_role = MockRole()
        self.default_role.mention = "@everyone"
        self.emojis = []

    def get_member(self, *args):
        return None


class MockReact:
    def __init__(self, message, emoji, user):
        self.message = message
        self.emoji = emoji
        self.count = 1
        self._users = [user]

    def users(self):
        class user:
            pass
        u = user()

        async def flatten():
            return self._users
        u.flatten = flatten
        return u


class MockMessage:
    def __init__(self, bot, id, sends, reaction_sends, guild_id, content=None, resp_id=0):
        self.bot = bot
        self.id = id
        self.sends = sends
        self.reaction_sends = reaction_sends
        self._state = MockChannel(bot, sends, reaction_sends, resp_id)
        self.guild = MockGuild(guild_id)
        self.author = MockMember()
        self.channel = MockChannel(bot, sends, reaction_sends, resp_id)
        self.content = content
        self.reactions = []

    async def add_reaction(self, emoji, bot=True):
        user = MockMember()
        if bot:
            self.reaction_sends.append((self.id, emoji))
            user = self.bot
        for react in self.reactions:
            if emoji == react.emoji:
                react._users.append(user)
                return react
        else:
            react = MockReact(self, emoji, user)
            self.reactions.append(react)
            return react

    async def remove_reaction(self, emoji):
        for react in self.reactions:
            if emoji == react.emoji:
                react._users = [self.bot.user]
                return react

    async def edit(self, content=None):
        # print("EDIT " + content)
        self.sends.append(content)
