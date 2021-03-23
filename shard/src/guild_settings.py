import json
from lib.models import Settings
from lib.config import FAKE_GUILD_IDS
from sqlalchemy.orm.exc import NoResultFound
from discord.ext.commands import Cog
import discord
from typing import List, Optional

RYTHMS_ID = 235088799074484224


class Setting:

    def __init__(self, session, guild):
        self.session = session
        self.guild_id = guild.id
        self.guild = guild
        self._settings_dict = self._load_from_db(self.guild_id)

    @property
    def command_prefix(self) -> str:
        return self._settings_dict['command_prefix'] if 'command_prefix' in self._settings_dict else '!'

    @command_prefix.setter
    def command_prefix(self, new_command_prefex: str):
        self._settings_dict['command_prefix'] = new_command_prefex
        self._update_db()

    @property
    def music_enabled(self) -> bool:
        return self._settings_dict['music_enabled'] if 'music_enabled' in self._settings_dict else not bool(
            self.guild.get_member(RYTHMS_ID))

    @music_enabled.setter
    def music_enabled(self, new_music_enabled: bool):
        self._settings_dict['music_enabled'] = new_music_enabled
        self._update_db()

    @property
    def music_role(self) -> Optional[discord.Role]:
        role_id = self._settings_dict.get('music_role_id', None)
        return self.guild.get_role(role_id)

    @music_role.setter
    def music_role(self, new_music_role: discord.Role):
        self._settings_dict['music_role_id'] = new_music_role.id
        self._update_db()

    @property
    def music_volume(self) -> float:
        return self._settings_dict.get('music_volume', 0.10)

    @music_volume.setter
    def music_volume(self, new_music_volume):
        self._settings_dict['music_volume'] = new_music_volume
        self._update_db()

    @property
    def scrim_channel_id(self) -> int:
        if 'scrim_channel_id' in self._settings_dict:
            return self._settings_dict.get('scrim_channel_id')
        channel = discord.utils.find(lambda c: c.name == 'lfs-posts', self.guild.channels)
        return channel.id if channel else 0

    @scrim_channel_id.setter
    def scrim_channel_id(self, scrim_channel_id: int):
        self._settings_dict['scrim_channel_id'] = scrim_channel_id
        self._update_db()

    @property
    def starboard_emoji(self) -> str:
        return self._settings_dict['starboard_emoji'] if 'starboard_emoji' in self._settings_dict else "⭐"

    @starboard_emoji.setter
    def starboard_emoji(self, new_emoji: str):
        self._settings_dict['starboard_emoji'] = new_emoji
        self._update_db()

    @property
    def pug_emoji(self) -> str:
        return self._settings_dict['pug_emoji'] if 'pug_emoji' in self._settings_dict else "✅"

    @pug_emoji.setter
    def pug_emoji(self, new_emoji: str):
        self._settings_dict['pug_emoji'] = new_emoji
        self._update_db()

    @property
    def pug_timeout_speed(self) -> int:
        return self._settings_dict['pug_timeout_speed'] if 'pug_timeout_speed' in self._settings_dict else 15

    @pug_timeout_speed.setter
    def pug_timeout_speed(self, new_timeout_speed: int):
        self._settings_dict['pug_timeout_speed'] = new_timeout_speed
        self._update_db()

    @property
    def responses_only_author_remove(self) -> bool:
        return self._settings_dict.get('responses_only_author_remove', False)

    @responses_only_author_remove.setter
    def responses_only_author_remove(self, only: bool) -> None:
        self._settings_dict['responses_only_author_remove'] = only
        self._update_db()

    @property
    def responses_whois_emoji(self) -> str:
        return self._settings_dict.get('responses_whois_emoji', '💬')

    @responses_whois_emoji.setter
    def responses_whois_emoji(self, new_emoji: str) -> None:
        self._settings_dict['responses_whois_emoji'] = new_emoji
        self._update_db()

    @property
    def responses_allow_embeds(self) -> bool:
        return self._settings_dict.get('responses_allow_embeds', True)

    @responses_allow_embeds.setter
    def responses_allow_embeds(self, new: bool) -> None:
        self._settings_dict['responses_allow_embeds'] = bool(new)
        self._update_db()

    @property
    def responses_allow_newlines(self) -> bool:
        return self._settings_dict.get('responses_allow_newlines', False)

    @responses_allow_newlines.setter
    def responses_allow_newlines(self, new: bool) -> None:
        self._settings_dict['responses_allow_newlines'] = bool(new)
        self._update_db()

    @property
    def responses_limit(self) -> int:
        return self._settings_dict['responses_limit'] if 'responses_limit' in self._settings_dict else None

    @responses_limit.setter
    def responses_limit(self, new_threshold: int):
        self._settings_dict['responses_limit'] = new_threshold
        self._update_db()

    @property
    def responses_trigger_length(self) -> int:
        return self._settings_dict.get('responses_trigger_length', 2)

    @responses_trigger_length.setter
    def responses_trigger_length(self, new_length: int) -> None:
        self._settings_dict['responses_trigger_length'] = new_length
        self._update_db()

    @property
    def responses_response_length(self) -> int:
        return self._settings_dict.get('responses_response_length', 500)

    @responses_response_length.setter
    def responses_response_length(self, new_length: int) -> None:
        self._settings_dict['responses_response_length'] = new_length
        self._update_db()

    @property
    def responses_allow_regex(self) -> bool:
        return self._settings_dict.get('responses_allow_regex', False)

    @responses_allow_regex.setter
    def responses_allow_regex(self, allow: bool) -> None:
        self._settings_dict['responses_allow_regex'] = allow
        self._update_db()

    @property
    def responses_allow_collision(self) -> bool:
        return self._settings_dict.get('responses_allow_collision', False)

    @responses_allow_collision.setter
    def responses_allow_collision(self, allow: bool) -> None:
        self._settings_dict['responses_allow_collision'] = allow
        self._update_db()

    @property
    def responses_enabled(self) -> bool:
        return self._settings_dict.get('responses_enabled', True)

    @responses_enabled.setter
    def responses_enabled(self, enabled: bool) -> None:
        self._settings_dict['responses_enabled'] = enabled
        self._update_db()

    @property
    def starboard_threshold(self) -> int:
        return self._settings_dict['starboard_threshold'] if 'starboard_threshold' in self._settings_dict else 5

    @starboard_threshold.setter
    def starboard_threshold(self, new_threshold: int):
        self._settings_dict['starboard_threshold'] = new_threshold
        self._update_db()

    @property
    def gulag_emoji(self) -> str:
        return self._settings_dict['gulag_emoji'] if 'gulag_emoji' in self._settings_dict else "⚒️"

    @gulag_emoji.setter
    def gulag_emoji(self, new_emoji: str):
        self._settings_dict['gulag_emoji'] = new_emoji
        self._update_db()

    @property
    def gulag_threshold(self) -> int:
        return self._settings_dict['gulag_threshold'] if 'gulag_threshold' in self._settings_dict else 5

    @gulag_threshold.setter
    def gulag_threshold(self, new_threshold: int):
        self._settings_dict['gulag_threshold'] = new_threshold
        self._update_db()

    @property
    def gulag_severity(self) -> int:
        return self._settings_dict['gulag_severity'] if 'gulag_severity' in self._settings_dict else 5

    @gulag_severity.setter
    def gulag_severity(self, new_severity: int):
        self._settings_dict['gulag_severity'] = new_severity
        self._update_db()

    @property
    def roles_dict(self) -> dict:
        roles = self._settings_dict['roles_dict'] if 'roles_dict' in self._settings_dict else {}
        # TODO db migration for str ids. can remove after every server has updated
        return {k: int(v) for k, v in roles.items()}

    @roles_dict.setter
    def roles_dict(self, roles_dict: dict):
        self._settings_dict['roles_dict'] = roles_dict
        self._update_db()

    @property
    def default_role_id(self) -> int:
        return int(self._settings_dict['default_role']) if 'default_role' in self._settings_dict else 0

    @default_role_id.setter
    def default_role_id(self, new_id: int):
        self._settings_dict['default_role'] = new_id
        self._update_db()

    @property
    def bot_commands_channels(self) -> List[int]:
        return self._settings_dict['bot_commands'] if 'bot_commands' in self._settings_dict else []

    @bot_commands_channels.setter
    def bot_commands_channels(self, new_bot_commands: List[int]):
        self._settings_dict['bot_commands'] = new_bot_commands
        self._update_db()

    @property
    def stats_exclude(self) -> List[int]:
        return list(set(self._settings_dict.get('stats_exclude', [])))

    @stats_exclude.setter
    def stats_exclude(self, new_excludes: List[int]) -> None:
        self._settings_dict['stats_exclude'] = new_excludes
        self._update_db()

    @property
    def admin_ids(self) -> List[int]:
        '''stupid alias'''
        return self.admins_ids

    @property
    def admins_ids(self) -> List[int]:
        default_admins = [self.guild.owner.id]
        default_admins += [m.id for role in self.guild.roles for m in role.members if role.permissions.administrator]
        default_admins += [self.guild.owner.id]

        return list(set(default_admins + [int(a) for a in self._settings_dict.get('admins', [])]))

    @admin_ids.setter
    def admin_ids(self, new_admins: List[int]):
        '''stupid alias'''
        self.admins_ids = new_admins

    @admins_ids.setter
    def admins_ids(self, new_admins: List[int]):
        self._settings_dict['admins'] = new_admins
        self._update_db()

    @property
    def bot_emoji(self) -> str:
        return self._settings_dict['bot_emoji'] if 'bot_emoji' in self._settings_dict else "🤖"

    @bot_emoji.setter
    def bot_emoji(self, new_emoji: str):
        self._settings_dict['bot_emoji'] = new_emoji
        self._update_db()

    @property
    def nice_emoji(self) -> str:
        return self._settings_dict['nice_emoji'] if 'nice_emoji' in self._settings_dict else "❤"

    @nice_emoji.setter
    def nice_emoji(self, new_emoji: str):
        self._settings_dict['nice_emoji'] = new_emoji
        self._update_db()

    @property
    def edit_emoji(self) -> str:
        return self._settings_dict['edit_emoji'] if 'edit_emoji' in self._settings_dict else "📝"

    @edit_emoji.setter
    def edit_emoji(self, new_emoji: str):
        self._settings_dict['edit_emoji'] = new_emoji
        self._update_db()

    @property
    def toxic_emoji(self) -> str:
        return self._settings_dict['toxic_emoji'] if 'toxic_emoji' in self._settings_dict else "👿"

    @toxic_emoji.setter
    def toxic_emoji(self, new_emoji: str):
        self._settings_dict['toxic_emoji'] = new_emoji
        self._update_db()

    @property
    def aut_emoji(self) -> str:
        return self._settings_dict['aut_emoji'] if 'aut_emoji' in self._settings_dict else "🅱"

    @aut_emoji.setter
    def aut_emoji(self, new_emoji: str):
        self._settings_dict['aut_emoji'] = new_emoji
        self._update_db()

    @property
    def repost_del_msg(self) -> bool:
        return self._settings_dict['repost_del_msg'] if 'repost_del_msg' in self._settings_dict else False

    @repost_del_msg.setter
    def repost_del_msg(self, new_setting: bool):
        self._settings_dict['repost_del_msg'] = new_setting
        self._update_db()

    @property
    def norm_emoji(self) -> str:
        return self._settings_dict['norm_emoji'] if 'norm_emoji' in self._settings_dict else "💤"

    @norm_emoji.setter
    def norm_emoji(self, new_emoji: str):
        self._settings_dict['norm_emoji'] = new_emoji
        self._update_db()

    @property
    def manage_emojis(self) -> bool:
        return self._settings_dict['manage_emojis'] if 'manage_emojis' in self._settings_dict else False

    @manage_emojis.setter
    def manage_emojis(self, manage_emojis: bool):
        self._settings_dict['manage_emojis'] = manage_emojis
        self._update_db()

    @property
    def emojis(self) -> dict:
        return self._settings_dict['emojis'] if 'emojis' in self._settings_dict else {}

    @emojis.setter
    def emojis(self, new_emojis: dict):
        self._settings_dict['emojis'] = new_emojis
        self._update_db()

    def _load_from_db(self, guild_id) -> dict:
        if self.guild_id < FAKE_GUILD_IDS:
            return {}
        settings_row = None
        try:
            settings_row = self.session.query(Settings).filter_by(server_id=int(guild_id)).one()
        except NoResultFound:
            new_guild = Settings(int(self.guild_id), json.dumps({}))
            self.session.add(new_guild)
        self.session.commit()
        return json.loads(settings_row.json_blob) if settings_row else {}

    def _update_db(self):
        if self.guild_id < FAKE_GUILD_IDS:
            return
        new_data = {
            'server_id': int(self.guild_id),
            'json_blob': json.dumps(self._settings_dict)
        }
        self.session.query(Settings).filter_by(server_id=int(self.guild_id)).update(new_data)
        self.session.commit()


class GuildSettings(Cog):

    def __init__(self, bot):
        self.guilds = {}
        self.session = bot.session

    def __getitem__(self, key):
        return self.get_guild(key)

    def get_guild(self, guild):
        if guild is None:
            return None
        try:
            return self.guilds[guild]
        except KeyError:
            self.guilds[guild] = Setting(self.session, guild)
            return self.guilds[guild]


def setup(bot):
    bot.add_cog(GuildSettings(bot))
