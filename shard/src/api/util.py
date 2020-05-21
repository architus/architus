from functools import wraps

from lib.status_codes import StatusCodes as sc


def fetch_guild(func):
    @wraps(func)
    async def decorator(self, *args, **kwargs):
        guild = self.bot.get_guild(args[0])

        if not guild:
            return {'message': "Unknown Guild"}, sc.NOT_FOUND_404

        return await func(self, guild, *args[1:], **kwargs)
    return decorator
