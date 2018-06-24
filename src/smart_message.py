from collections import deque
from discord import message

class smart_message:
    def __init__(self, message):
        self.most_recent = message
        self.edits = deque([], maxlen=10)
        self.edits.append(message)
        self.popup = None

    def add_edit(self, before, after):
        if (before.id == self.most_recent.id):
            self.edits.append(after)
            self.most_recent = after;
            return True

    def peek(self):
        return self.most_recent

    def has_popup(self):
        return self.popup is None
    def add_popup(self):
        title = "last %d edits" % (len(self.edits))
        lem = list_embed(title, "date here?", self.most_recent.author)
        for edit in self.edits:
            lem.add("blah", edit.content)
        return get_embed
