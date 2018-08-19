import random
import emoji as emojitool
import re

class smart_command:
    def __init__(self, trigger, response, count, server):
        self.raw_trigger = self.filter_trigger(trigger)
        self.raw_response = emojitool.demojize(response)
        self.count = count
        self.server = server

    def triggered(self, phrase):
        return self.raw_trigger == self.filter_trigger(phrase)

    def generate_reacts(self):
        rereact = re.compile("\[(:.+?:)\]")
        matches = rereact.findall(self.raw_response)
        emojis = []
        for match in matches:
            emojis.append(self.get_custom_emoji(match))
        return emojis

    def get_custom_emoji(self, emojistr):
        for emoji in self.server.emojis:
            if ':'+emoji.name+':' == emojistr:
                return emoji
        return emojitool.emojize(emojistr, use_aliases=True)


    def generate_response(self):
        self.count += 1
        resp = self.raw_response
        renoun = re.compile("\[noun\]", re.IGNORECASE)
        readj = re.compile("\[adj\]", re.IGNORECASE)
        readv = re.compile("\[adv\]", re.IGNORECASE)
        reowl = re.compile("\[owl\]", re.IGNORECASE)
        recount = re.compile("\[count\]", re.IGNORECASE)
        remember = re.compile("\[member\]", re.IGNORECASE)
        relist = re.compile("\[([^,]+(,.*?)+)\]", re.IGNORECASE)
        rereact = re.compile("\[:.*:\]")


        while renoun.search(resp):
            resp = renoun.sub(get_noun(), resp, 1)
        while readj.search(resp):
            resp = readj.sub(get_adj(), resp, 1)
        while readv.search(resp):
            resp = readv.sub(get_adv(), resp, 1)
        while reowl.search(resp):
            resp = reowl.sub(get_owl(), resp, 1)
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

        return emojitool.emojize(resp)
    
    def filter_trigger(self, trigger):
        unicode_filter = re.compile('[\W_]+', re.UNICODE)
        filtered_trigger = unicode_filter.sub('', trigger)
        if (trigger[0] == '!' or trigger[0] == '?'):
            filtered_trigger = trigger[0] + filtered_trigger
        return filtered_trigger.lower()

    def __eq__(self, other):
        return self.raw_trigger == other.raw_trigger

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

#print(get_adj() + " " + get_noun())
#sc = smart_command("no u !@#$", "raines has sucked a [aDj] [Noun] [count] times [:dansgame:]", 10, None)
#print(sc.raw_trigger)
#print(sc.generate_response())
#print(sc.triggered('no  !!!!!!!!!!!!!!u!!!'))
