from datetime import datetime
from pytz import timezone

import discord

import os
import warnings
import asyncio
import itertools
import logging
import threading


_get_running_loop = getattr(asyncio, "get_running_loop", asyncio.get_event_loop)
logger = logging.getLogger(__name__)


class _Py38ThreadedChildWatcher(asyncio.AbstractChildWatcher):
    def __init__(self):
        self._pid_counter = itertools.count(0)
        self._threads = {}

    def is_active(self):
        return True

    def close(self):
        pass

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        pass

    def __del__(self, _warn=warnings.warn):
        threads = [t for t in list(self._threads.values()) if t.is_alive()]
        if threads:
            _warn(
                f"{self.__class__} has registered but not finished child processes",
                ResourceWarning,
                source=self
            )

    def add_child_handler(self, pid, callback, *args):
        loop = _get_running_loop()
        thread = threading.Thread(
            target=self._do_waitpid,
            name=f"waitpid-{next(self._pid_counter)}",
            args=(loop, pid, callback, args),
            daemon=True
        )
        self._threads[pid] = thread
        thread.start()

    def remove_child_handler(self, pid):
        return True

    def attach_loop(self, loop):
        pass

    def _do_waitpid(self, loop, expected_pid, callback, args):
        assert expected_pid > 0

        try:
            pid, status = os.waitpid(expected_pid, 0)
        except ChildProcessError:
            pid = expected_pid
            returncode = 255
            logger.warning(
                "Unknown child process pid %d, will report returncode 255", pid
            )
        else:
            if os.WIFSIGNALED(status):
                returncode = -os.WTERMSIG(status)
            elif os.WIFEXITED(status):
                returncode = os.WEXITSTATUS(status)
            else:
                returncode = status

            if loop.get_debug():
                logger.debug(
                    "process %s exited with returncode %s", expected_pid, returncode
                )

        if loop.is_closed():
            logger.warning("Loop %r that handles pid %r is closed", loop, pid)
        else:
            loop.call_soon_threadsafe(callback, pid, returncode, *args)

        self._threads.pop(expected_pid)


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
