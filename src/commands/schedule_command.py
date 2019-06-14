from src.commands.abstract_command import abstract_command
import time
import datetime
import pytz
import discord
import dateutil.parser

class schedule_command(abstract_command):

    YES_EMOJI = '✅'
    NO_EMOJI = '❌'
    MAYBE_EMOJI = '🤷'

    def __init__(self):
        super().__init__("schedule")

    async def exec_cmd(self, **kwargs):
        # event bot's id
        if self.server.get_member('476042677440479252'):
            print("not scheduling cause event bot exists")
            return
        region = self.server.region
        print(region)
        tz = pytz.timezone(self.get_timezone(region))
        ct = datetime.datetime.now(tz=tz)
        del self.args[0]
        title = []
        parsed_time = None
        for i in range(len(self.args)):
            try:
                print(' '.join(self.args))
                parsed_time = dateutil.parser.parse(' '.join(self.args))
                parsed_time = tz.localize(parsed_time)
                break
            except Exception as e:
                pass
            title.append(self.args[0])
            del self.args[0]

        if not parsed_time:
            parsed_time = await self.prompt_date(self.author)
            if not parsed_time: return
            parsed_time = tz.localize(parsed_time)
        if len(title) == 0:
            title_str = await self.prompt_title(self.author)
            if not title_str: return
        else:
            title_str = ' '.join(title)
        
        yes = []
        no = []
        maybe = []
        msg = await self.client.send_message(self.channel, self.render_text(title_str, parsed_time, yes, no, maybe))
        await self.client.add_reaction(msg, self.YES_EMOJI)
        await self.client.add_reaction(msg, self.NO_EMOJI)
        await self.client.add_reaction(msg, self.MAYBE_EMOJI)
        while True:
            react = await self.client.wait_for_reaction([self.YES_EMOJI, self.NO_EMOJI, self.MAYBE_EMOJI], message=msg)
            if react.user == self.client.user: continue
            elif self.YES_EMOJI in str(react.reaction.emoji) and react.user not in yes:
                yes.append(react.user)
                try: no.remove(react.user)
                except: pass
                try: maybe.remove(react.user)
                except: pass
            elif self.NO_EMOJI in str(react.reaction.emoji) and react.user not in no:
                no.append(react.user)
                try: yes.remove(react.user)
                except: pass
                try: maybe.remove(react.user)
                except: pass
            elif self.MAYBE_EMOJI in str(react.reaction.emoji) and react.user not in maybe:
                maybe.append(react.user)
                try: no.remove(react.user)
                except: pass
                try: yes.remove(react.user)
                except: pass

            await self.client.edit_message(msg, self.render_text(title_str, parsed_time, yes, no, maybe))

        return True


    def render_text(self, title_str, parsed_time, yes, no, maybe):
        return "__**%s**__\n**Time: **%s\n:white_check_mark: **Yes (%d): %s**\n:x: **No (%d): %s**\n:shrug: **Maybe (%d): %s**" % (title_str, parsed_time.strftime("%b %d %I:%M%p %Z"), len(yes), ' '.join([u.mention for u in yes]), len(no), ' '.join([u.mention for u in no]), len(maybe), ' '.join([u.mention for u in maybe]))

    async def prompt_date(self, author):
        await self.client.send_message(self.channel, "what time?")
        time_msg = await self.client.wait_for_message(timeout=30, author=author)
        try:
            return dateutil.parser.parse(time_msg.clean_content)
        except:
            await self.client.send_message(self.channel, "not sure what that means")
            return None

    async def prompt_title(self, author):
        await self.client.send_message(self.channel, "what event?")
        title_msg = await self.client.wait_for_message(timeout=30, author=author)
        return title_msg.clean_content or None

    def get_help(self, **kwargs):
        return "Start an event poll with pretty formatting. Knows the difference between daylight and standard time."

    def get_brief(self):
        return "Start an event poll with pretty formatting"

    def get_usage(self):
        return "<title> [date]"

    def get_timezone(self, region):
        region = str(region)
        if region == 'us-south' or region == 'us-east':
            return 'America/New_York'
        elif region == 'us-central':
            return 'America/Chicago'
        elif region == 'us-west':
            return 'America/Los_Angeles'
        else:
            return 'Etc/UTC'
