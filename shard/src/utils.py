from datetime import datetime
from pytz import timezone
from aiohttp import ClientSession
import io
import functools
from threading import Lock
from string import digits

import discord

from lib.config import logger
from lib.ipc import manager_pb2 as message


class TCPLock:
    """
    A basic thread safe TCP socket.
    """
    def __init__(self, s):
        self.connection = s
        self.lock = Lock()

    def write(self, b):
        self.lock.acquire()
        self.connection.send(b)
        self.lock.release()

    def send(self, b):
        self.write(b)

    def recv(self, num_bytes):
        self.lock.acquire()
        msg = self.connection.recv(num_bytes)
        self.lock.release()
        return msg


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


def channel_to_dict(ch) -> dict:
    # TODO
    return {'id': str(ch.id), 'name': ch.name}


def guild_to_dict(guild: discord.Guild) -> dict:
    params = (
        'id', 'name', 'icon', 'splash', 'owner_id', 'region', 'afk_timeout', 'unavailable',
        'max_members', 'banner', 'description', 'mfa_level', 'features', 'premium_tier',
        'premium_subscription_count', 'preferred_locale', 'member_count',
    )
    data = {p: getattr(guild, p) for p in params}
    data['id'] = str(data['id'])
    return data


def guild_to_message(guild: discord.Guild):
    return message.Guild(
        id=guild.id,
        name=guild.name,
        icon=guild.icon,
        splash=guild.splash,
        owner_id=guild.owner_id,
        region=repr(guild.region),
        afk_timeout=guild.afk_timeout,
        unavailable=guild.unavailable,
        max_members=guild.max_members,
        banner=guild.banner,
        description=guild.description,
        mfa_level=guild.mfa_level,
        features=guild.features,
        premium_tier=guild.premium_tier,
        premium_subscription_count=guild.premium_subscription_count,
        preferred_locale=guild.preferred_locale,
        member_count=guild.member_count
    )


def user_to_dict(user: discord.User) -> dict:
    params = ('id', 'name', 'avatar', 'discriminator')
    data = {p: getattr(user, p) for p in params}
    data['id'] = str(data['id'])
    return data


def member_to_dict(member: discord.Member) -> dict:
    params = ('id', 'name', 'nick', 'avatar', 'discriminator')
    data = {p: getattr(member, p) for p in params}
    data['id'] = str(data['id'])
    data['roles'] = [str(r.id) for r in member.roles]
    data['color'] = str(member.color)
    data['joined_at'] = member.joined_at.isoformat()
    logger.debug(data)
    return data


def role_to_dict(role: discord.Role) -> dict:
    params = ('id', 'name', 'hoist', 'position', 'managed', 'mentionable')
    data = {p: getattr(role, p) for p in params}
    data['id'] = str(data['id'])
    data['members'] = [str(m.id) for m in role.members]
    data['color'] = str(role.color)
    return data


def bot_commands_only(cmd):
    """Adds the restriction that a command must be run in a bot commands channel if the guild has one set
    Does nothing if not in a guild
    Requires that the command be in the context of a cog that has the attribute `self.bot`
    """
    @functools.wraps(cmd)
    async def bc_cmd(self, ctx, *args, **kwargs):
        if ctx.guild:
            settings = self.bot.settings[ctx.guild]
            if settings.bot_commands_channels\
                    and ctx.channel.id not in settings.bot_commands_channels\
                    and ctx.author.id not in settings.admin_ids:

                for channel_id in settings.bot_commands_channels:
                    bc_ch = discord.utils.get(ctx.guild.channels, id=channel_id)
                    if bc_ch:
                        await ctx.send(f"Please use {bc_ch.mention} for that command")
                        return
        return await cmd(self, ctx, *args, **kwargs)
    return bc_cmd


def mention_to_name(guild: discord.Guild, mention: str) -> str:
    if mention[0] != '<':
        raise ValueError(f"this doesn't look like a mention str: {mention}")
    id = int("".join([n for n in mention if n in digits]))
    if mention[1] == '@':
        member = guild.get_member(id)
        if member is not None:
            return f"@{member.display_name}"
    elif mention[1] == '&':
        rl = guild.get_role(id)
        if rl is not None:
            return f"@{rl.name}"
    elif mention[1] == '#':
        ch = guild.get_channel(id)
        if ch is not None:
            return f"#{ch.name}"
    return mention
