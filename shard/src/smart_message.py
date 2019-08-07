from collections import deque
from src.list_embed import ListEmbed
from pytz import timezone


class smart_message:
    def __init__(self, message):
        self.most_recent = message
        self.edits = deque([], maxlen=10)
        dumb = dumb_message(message.content, message.author, message.id, message.created_at)
        self.edits.append(dumb)
        self.ogtime = self.get_datetime(message.created_at)
        self.popup = None
        self.embed = None

    async def edit_popup(self):
        if self.has_popup:
            lem = self.get_popup_embed()
            await self.popup.edit(embed=lem)

    async def add_popup(self, ctx):
        if not self.has_popup:
            lem = self.get_popup_embed()
            popup = await ctx.send(embed=lem)
            self.popup = popup
        else:
            await self.edit_popup()

    async def delete_popup(self):
        if self.has_popup:
            await self.popup.delete()
            self.popup = None

    def add_edit(self, before, after, timestamp):
        if (before.id == self.most_recent.id):
            dumb_after = dumb_message(after.content, after.author, after.id, timestamp)
            self.edits.append(dumb_after)
            self.most_recent = dumb_after
            return True

    def peek(self):
        return self.most_recent

    @property
    def has_popup(self):
        return self.popup is not None

    def get_popup_embed(self):
        title = "last %d edits" % (len(self.edits))
        lem = ListEmbed(title, self.ogtime.strftime("%m-%d %I:%M %p"), self.most_recent.author)
        for edit in self.edits:
            est = self.get_datetime(edit.timestamp)
            lem.add(est.strftime("%I:%M:%S %p"), edit.content)
        return lem.get_embed()

    def get_datetime(self, timestamp):
        utc = timestamp.replace(tzinfo=timezone('UTC'))
        est = utc.astimezone(timezone('US/Eastern'))
        return timestamp


class dumb_message:
    def __init__(self, message, author, mid, timestamp):
        self.content = message
        self.author = author
        self.id = mid
        self.timestamp = timestamp
