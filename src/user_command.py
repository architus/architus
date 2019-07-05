import random
import emoji as emojitool
import re
from sqlalchemy.exc import IntegrityError
from src.models import Command
GROUP_LIMIT = 1


def update_command(session, triggerkey, response, count, guild, author_id, delete=False):
    if guild.id < 1000000:
        return
    if (delete):
        session.query(Command).filter_by(trigger=str(guild.id) + triggerkey).delete()
        session.commit()
        return
    new_data = {
        'server_id': guild.id,
        'response': response,
        'count': count,
        'author_id': int(author_id)
    }
    session.query(Command).filter_by(trigger=str(guild.id) + triggerkey).update(new_data)
    session.commit()


class UserCommand:
    def __init__(self, session, trigger, response, count, guild, author_id, new=False):
        self.session = session
        self.raw_trigger = self.filter_trigger(trigger)
        self.raw_response = emojitool.demojize(response)
        self.capture_regex = ''
        if '*' in trigger:
            self.capture_regex = self.generate_capture_regex()
        self.count = count
        self.server = guild
        self.author_id = author_id
        if new and self.validate_new_command() and guild.id > 1000000:
            try:
                new_command = Command(str(guild.id) + self.raw_trigger, self.raw_response, count, guild.id, author_id)
                self.session.add(new_command)
                self.session.commit()
            except IntegrityError:
                self.session.rollback()
                raise DuplicatedTriggerException(self.raw_trigger)


    def validate_new_command(self):
        if len(self.raw_trigger) < 1:
            raise ShortTriggerException("Please use a longer trigger")
        if len(self.raw_response) > 200:
            raise LongTriggerException("That response is too long, ask an admin to set it")
        if self.raw_response in ("author", "list", "remove"):
            raise ResponseKeywordException()
        # if user has too many commands?
        # language filter?
        return True

    async def execute(self, message):
        resp = self.generate_response(message.author, message.content)
        update_command(self.session, self.raw_trigger, self.raw_response, self.count, self.server, self.author_id)
        reacts = self.generate_reacts()
        if resp:
            await message.channel.send(resp)
        for react in reacts:
            await message.add_reaction(react)

    def triggered(self, phrase):
        if self.capture_regex:
            capture = re.compile(self.capture_regex, re.IGNORECASE)
            return capture.search(self.filter_trigger(phrase))
        else:
            return self.raw_trigger == self.filter_trigger(phrase)

    def generate_reacts(self):
        rereact = re.compile(r"\[(:.+?:)\]")
        matches = rereact.findall(self.raw_response)
        emojis = []
        for match in matches:
            emojis.append(self.get_custom_emoji(match))
        return emojis

    def get_custom_emoji(self, emojistr):
        for emoji in self.server.emojis:
            if ':' + emoji.name + ':' == emojistr:
                return emoji
        return emojitool.emojize(emojistr, use_aliases=True)

    def generate_capture_regex(self):
        regex = '^'
        count = 0
        if len(self.raw_trigger) < 4:
            raise VaguePatternError

        for char in self.raw_trigger:
            if char == '*':
                if count < GROUP_LIMIT:
                    regex += '(.*?)\\s?'
                    count += 1
            else:
                regex += char + '\\s*'
        regex += '$'
        return regex

    def generate_response(self, author, real_trigger):
        cap = ''
        if self.capture_regex:
            capture = re.compile(self.capture_regex, re.IGNORECASE)
            cap = capture.search(real_trigger)
            if cap and cap.group(1):
                cap = cap.group(1)
            else:
                cap = ''

        # self.generate_capture_regex()
        self.count += 1
        resp = self.raw_response
        renoun = re.compile(r"\[noun\]", re.IGNORECASE)
        readj = re.compile(r"\[adj\]", re.IGNORECASE)
        readv = re.compile(r"\[adv\]", re.IGNORECASE)
        recapture = re.compile(r"\[capture\]", re.IGNORECASE)
        reowl = re.compile(r"\[owl\]", re.IGNORECASE)
        recount = re.compile(r"\[count\]", re.IGNORECASE)
        remember = re.compile(r"\[member\]", re.IGNORECASE)
        reauthor = re.compile(r"\[author\]", re.IGNORECASE)
        relist = re.compile(r"\[([^,]+(,.*?)+)\]", re.IGNORECASE)
        rereact = re.compile(r"\[:.*:\]")

        reclean = re.compile(r"@((:?everyone)|(:?channel)|(:?here))", re.IGNORECASE)

        while renoun.search(resp):
            resp = renoun.sub(get_noun(), resp, 1)
        while readj.search(resp):
            resp = readj.sub(get_adj(), resp, 1)
        while readv.search(resp):
            resp = readv.sub(get_adv(), resp, 1)
        while reowl.search(resp):
            resp = reowl.sub(get_owl(), resp, 1)
        if recapture.search(resp):
            resp = recapture.sub(cap, resp, 1)
        while reauthor.search(resp):
            unicode_filter = re.compile('[^a-zA-Z0-9* ]+', re.UNICODE)
            filtered_name = unicode_filter.sub('', author.display_name)
            resp = reauthor.sub(filtered_name, resp, 1)
        while remember.search(resp):
            resp = remember.sub(random.choice(list(self.server.members)).display_name, resp, 1)
        resp = recount.sub(str(self.count), resp)
        while rereact.search(resp):
            resp = rereact.sub('', resp, 1)

        custom_list = relist.search(resp)
        while custom_list:
            things = custom_list.group(1).split(',')
            resp = relist.sub(random.choice(things).lstrip(), resp, 1)
            custom_list = relist.search(resp)

        unclean = reclean.search(resp)
        while unclean:
            resp = reclean.sub(unclean.group(1), resp, 1)
            unclean = reclean.search(resp)

        return emojitool.emojize(resp)

    def filter_trigger(self, trigger):
        if len(trigger) == 0:
            return ''
        unicode_filter = re.compile('[^a-zA-Z0-9*]+', re.UNICODE)
        filtered_trigger = unicode_filter.sub('', trigger)
        if (trigger[0] == '!' or trigger[0] == '?'):
            filtered_trigger = trigger[0] + filtered_trigger
        return filtered_trigger.lower()

    def __eq__(self, other):
        return self.raw_trigger.replace('*', '') == other.raw_trigger.replace('*', '')

    def __str__(self):
        return self.raw_trigger + '::' + self.raw_response

    def __gt__(self, other):
        return '*' in self.raw_trigger and '*' not in other.raw_trigger

    def __lt__(self, other):
        return '*' in other.raw_trigger and '*' not in self.raw_trigger


def get_noun():
    fname = "res/words/nouns.txt"
    nouns = []
    with open(fname) as f:
        nouns = list(set(noun.strip() for noun in f))
    return random.choice(nouns)


def get_adv():
    fname = "res/words/adverbs.txt"
    advs = []
    with open(fname) as f:
        advs = list(set(adv.strip() for adv in f))
    return random.choice(advs)


def get_adj():
    fname = "res/words/adjectives.txt"
    adjs = []
    with open(fname) as f:
        adjs = list(set(adj.strip() for adj in f))
    return random.choice(adjs)


def get_owl():
    fname = "res/words/owl.txt"
    owls = []
    with open(fname) as f:
        owls = list(set(owl.strip() for owl in f))
    return random.choice(owls)


class VaguePatternError(Exception):
    pass


class ShortTriggerException(Exception):
    pass


class LongTriggerException(Exception):
    pass


class DuplicatedTriggerException(Exception):
    pass


class ResponseKeywordException(Exception):
    pass
