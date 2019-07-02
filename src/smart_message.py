from collections import deque
from src.list_embed import list_embed
from pytz import timezone


class smart_message:
    def __init__(self, message):
        self.most_recent = message
        self.edits = deque([], maxlen=10)
        dumb = dumb_message(message.content, message.author, message.id, message.timestamp)
        self.edits.append(dumb)
        self.ogtime = self.get_datetime(message.timestamp)
        self.popup = None
        self.embed = None

    def add_edit(self, before, after):
        if (before.id == self.most_recent.id):
            dumb_after = dumb_message(after.content, after.author, after.id, after.edited_timestamp)
            self.edits.append(dumb_after)
            self.most_recent = dumb_after
            return True

    def peek(self):
        return self.most_recent

    def has_popup(self):
        return self.popup is not None

    def add_popup(self):
        title = "last %d edits" % (len(self.edits))
        lem = list_embed(title, self.ogtime.strftime("%m-%d %I:%M %p"), self.most_recent.author)
        for edit in self.edits:
            est = self.get_datetime(edit.timestamp)
            lem.add(est.strftime("%I:%M:%S %p"), edit.content)
        return lem.get_embed()

    def get_datetime(self, timestamp):
        if timestamp is None:
            return timestamp
        utc = timestamp.replace(tzinfo=timezone('UTC'))
        est = utc.astimezone(timezone('US/Eastern'))
        return est


class dumb_message:
    def __init__(self, message, author, mid, timestamp):
        self.content = message
        self.author = author
        self.id = mid
        self.timestamp = timestamp
