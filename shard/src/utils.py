from datetime import datetime
from pytz import timezone
from aiohttp import ClientSession
import io

import discord

from lib.config import logger


async def download_emoji(emoji: discord.Emoji) -> io.BytesIO:
    async with ClientSession() as session:
        async with session.get(str(emoji.url)) as resp:
            if resp.status == 200:
                buf = io.BytesIO()
                buf.write(await resp.read())
                buf.seek(0)
                return buf
    logger.debug("API gave unexpected response (%d) emoji not saved" % resp.status)
    return None


async def send_message_webhook(channel, content, avatar_url=None, username=None, embeds=None):
    webhooks = await channel.webhooks()
    if webhooks:
        webhook = webhooks[0]
    else:
        webhook = await channel.create_webhook(name="architus webhook")
    await webhook.send(content=content, avatar_url=avatar_url, username=username, embeds=embeds)


def timezone_aware_format(time: datetime, timezone_str: str = 'US/Eastern') -> str:
    utc = time.replace(tzinfo=timezone('UTC'))
    tz = utc.astimezone(timezone(timezone_str))
    return tz.strftime("%Y-%m-%d %I:%M %p")


def guild_to_dict(guild: discord.Guild) -> dict:
    parameters = (
        'id', 'name', 'icon', 'splash', 'owner_id', 'region', 'afk_timeout', 'unavailable',
        'max_members', 'banner', 'description', 'mfa_level', 'features', 'premium_tier',
        'premium_subscription_count', 'preferred_locale', 'member_count',
    )
    return {p: getattr(guild, p) for p in parameters}
