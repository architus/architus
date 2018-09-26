from src.commands.abstract_command import abstract_command
import time
import datetime
import pytz
from discord import ServerRegion
import discord

class schedule_command(abstract_command):

    def __init__(self):
        super().__init__("schedule")

    async def exec_cmd(self, **kwargs):
        region = self.server.region
        print(type(region))
        import datetime
        tz = pytz.timezone(self.get_timezone(region))
        ct = datetime.datetime.now(tz=tz)
        await self.client.send_message(self.channel, ct.isoformat())

    def get_help(self):
        return "!gulag <@member> - hold a gulag vote"

    def get_usage(self):
        return "!gulag <@member> - hold a gulag vote"

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
