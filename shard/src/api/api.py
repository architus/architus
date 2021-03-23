import secrets
from typing import List
from asyncio import create_task
import re

from discord.ext.commands import Cog, Context
import discord

from lib.status_codes import StatusCodes as sc
from lib.pool_types import PoolType
from lib.config import logger, FAKE_GUILD_IDS
from src.auto_response import GuildAutoResponses
from src.api.util import fetch_guild
from src.api.pools import Pools
from src.api.mock_discord import MockMember, MockMessage, LogActions, MockGuild
from lib.ipc import manager_pb2 as message
from src.utils import guild_to_dict, lavasong_to_dict

url_rx = re.compile(r'https?://(?:www\.)?.+')


class Api(Cog):

    def __init__(self, bot):
        self.bot = bot
        self.fake_messages = {}
        self.pools = Pools(bot)

    async def api_entry(self, method_name, *args, **kwargs):
        """Callback method for the rpc server

        :param method_name: name of the method to execute
        :param *args: args to pass through
        :param **kwargs: kwargs to pass through
        """
        try:
            assert not method_name.startswith('_')
            method = getattr(self, method_name)
        except (AttributeError, AssertionError):
            logger.warning(f"Someone tried to call '{method}' but it doesn't exist (or is private)")
            return {"message": "No such method"}, sc.NOT_FOUND_404

        try:
            return await method(*args, **kwargs)
        except Exception as e:
            logger.exception(f"caught exception while handling remote request")
            return {"message": f"'{e}'"}, sc.INTERNAL_SERVER_ERROR_500

    async def ping(self):
        return {'message': 'pong'}, sc.OK_200

    async def guild_count(self):
        try:
            resp = await self.bot.manager_client.guild_count(message.GuildCountRequest())
            return {'guild_count': resp.guild_count, 'user_count': resp.user_count}, sc.OK_200
        except Exception:
            logger.info(f"Shard {self.bot.shard_id} failed to get guild count from manager")
            return {'guild_count': -1, 'user_count': -1}, sc.INTERNAL_SERVER_ERROR_500

    async def all_guilds(self):
        all_guilds = []
        for g in await self.bot.manager_client.all_guilds(message.AllGuildsRequest()):
            all_guilds.append({
                'id': g.id,
                'name': g.name,
                'icon': g.icon,
                'region': g.region,
                'description': g.description,
                'preferred_locale': g.preferred_locale,
                'member_count': g.member_count,
            })
        return {'guilds': all_guilds}, sc.OK_200

    async def set_response(self, user_id, guild_id, trigger, response):
        return {'message': 'unimplemented'}, 500

    async def get_playlist(self, guild_id):
        voice = self.bot.lavalink.player_manager.get(guild_id)
        if voice is None:
            return {}, sc.OK_200
        else:
            dicts = [lavasong_to_dict(s) for s in voice.queue]
            return {'playlist': dicts}, sc.OK_200

    @fetch_guild
    async def queue_song(self, guild, uid, song):
        lava_cog = self.bot.cogs['Voice']
        if guild is None:
            return {}, sc.BAD_REQUEST_400
        user = guild.get_member(uid)
        if user is None:
            return {}, sc.BAD_REQUEST_400

        try:
            await lava_cog.ensure_voice(user, guild, True)
        except discord.CommandInvokeError:
            return {}, sc.UNAUTHORIZED_401

        added_songs = await lava_cog.enqueue(song, user, guild)
        if added_songs == []:
            return {}, sc.NOT_FOUND_404
        elif added_songs[0] == 'playlist':
            return {'playlist': added_songs}, sc.OK_200
        else:
            return {lavasong_to_dict(added_songs[1])}, sc.OK_200

    async def users_guilds(self, user_id):
        users_guilds = []
        for guild in self.bot.guilds:
            member = guild.get_member(int(user_id))
            if member is not None:
                settings = self.bot.settings[guild]

                g = guild_to_dict(guild)
                g.update({
                    "has_architus": True,
                    "architus_admin": int(user_id) in settings.admins_ids,
                    'permissions': member.guild_permissions.value,
                })
                users_guilds.append(g)
        return users_guilds, sc.OK_200

    async def is_member(self, user_id, guild_id):
        '''check if user is a member or admin of the given guild'''
        guild = self.bot.get_guild(int(guild_id))
        if not guild:
            return {'member': False, 'admin': False}, sc.OK_200
        settings = self.bot.settings[guild]
        member = guild.get_member(int(user_id))
        return {
            'member': bool(member),
            'admin': int(user_id) in settings.admins_ids,
            'permissions': member.guild_permissions.value if member else 0,
        }, sc.OK_200

    async def get_permissions(self, user_id: int, guild_id: int):
        guild = self.bot.get_guild(int(guild_id))
        settings = self.bot.settings[guild]
        default = not guild or not settings and user_id not in settings.admin_ids
        return {'permissions': 274 if default else 65535}

    async def delete_response(self, user_id, guild_id, trigger):
        return {'message': "No such command."}, sc.NOT_FOUND_404

    async def fetch_user_dict(self, id):
        usr = self.bot.get_user(int(id))
        if usr is None:
            return {'message': "No such user"}, sc.NOT_FOUND_404
        return {
            'name': usr.name,
            'avatar': usr.avatar,
            'discriminator': usr.discriminator
        }, sc.OK_200

    async def get_emoji(self, id):
        e = self.bot.get_emoji(int(id))
        if e is None:
            return {'message': "No such emoji"}, sc.NOT_FOUND_404
        return {
            'name': e.name,
            'url': str(e.url)
        }, sc.OK_200

    async def get_guild_emojis(self, guild_id):
        emoji_manager = self.bot.cogs['Emoji Manager'].managers[guild_id]
        return {'emojis': [{
            'id': str(e.id),
            'name': e.name,
            'authorId': str(e.author_id) if e.author_id is not None else None,
            'loaded': e.loaded,
            'numUses': e.num_uses,
            'discordId': str(e.discord_id),
            'url': await e.url(),
        } for e in emoji_manager.emojis]}, sc.OK_200

    async def get_extensions(self):
        return {'extensions': [k for k in self.bot.extensions.keys()]}, sc.OK_200

    async def reload_extension(self, extension_name):
        name = extension_name.replace('-', '.')
        try:
            self.bot.reload_extension(name)
        except discord.ext.commands.errors.ExtensionNotLoaded as e:
            logger.exception("Couldn't load extension")
            return {"message": f"Extension Not Loaded: {e}"}, sc.SERVICE_UNAVAILABLE_503
        return {"message": "Reload signal sent"}, sc.OK_200

    @fetch_guild
    async def bin_messages(self, guild, member_id):
        stats_cog = self.bot.cogs["Server Statistics"]
        emoji_manager = self.bot.cogs["Emoji Manager"].managers[guild.id]
        data = stats_cog.cache.get(guild.id, None)
        member = guild.get_member(member_id)
        if data is None or member is None:
            return {'message': "unknown member or guild"}, sc.NOT_FOUND_404
        return {
            'member_count': data.member_count,
            'architus_count': data.architus_count(member),
            'message_count': data.message_count(member),
            'common_words': data.common_words(member),
            'mention_counts': data.mention_counts(member),
            'member_counts': data.member_counts(member),
            'channel_counts': data.channel_counts(member),
            'time_member_counts': data.times_as_strings(member),
            'up_to_date': data.up_to_date,
            'forbidden': data.forbidden,
            'last_activity': data.last_activity(member).isoformat(),
            'popular_emojis': [str(e.id) for e in emoji_manager.emojis[:10]],
        }, sc.OK_200

    @fetch_guild
    async def get_guild_data(self, guild):
        return {
            'name': guild.name,
            'member_count': guild.member_count,
        }, sc.OK_200

    @fetch_guild
    async def load_emoji(self, guild: discord.Guild, emoji_id: int, member_id: int):
        emoji_manager = self.bot.cogs['Emoji Manager'].managers[guild.id]
        emoji = emoji_manager.find_emoji(a_id=emoji_id)
        if emoji is None:
            return {'message': "unknown emoji"}, sc.BAD_REQUEST_400
        await emoji_manager.load_emoji(emoji)
        return {'message': "successfully loaded"}, sc.OK_200

    @fetch_guild
    async def cache_emoji(self, guild: discord.Guild, emoji_id: int, member_id: int):
        emoji_manager = self.bot.cogs['Emoji Manager'].managers[guild.id]
        emoji = emoji_manager.find_emoji(a_id=emoji_id)
        if member_id not in self.bot.settings[guild].admin_ids:
            return {'message': "only admins may manually cache emoji"}, sc.UNAUTHORIZED_401
        if emoji is None:
            return {'message': "unknown emoji"}, sc.BAD_REQUEST_400
        await emoji_manager.cache_emoji(emoji)
        return {'message': "successfully cached"}, sc.OK_200

    @fetch_guild
    async def delete_emoji(self, guild: discord.Guild, emoji_id: int, member_id: int):
        member = guild.get_member(member_id)
        emoji_manager = self.bot.cogs['Emoji Manager'].managers[guild.id]
        emoji = emoji_manager.find_emoji(a_id=emoji_id)
        if emoji is None:
            return {'message': "unknown emoji"}, sc.BAD_REQUEST_400
        if emoji.author_id != member.id and member.id not in self.bot.settings[guild].admin_ids:
            return {'message': "you must own this emoji or have admin permissions"}, sc.UNAUTHORIZED_401
        await emoji_manager.delete_emoji(emoji)
        return {'message': "successfully deleted"}, sc.OK_200

    @fetch_guild
    async def settings_access(self, guild, setting=None, value=None):
        settings = self.bot.settings[guild]
        if hasattr(settings, setting):
            return {'value': getattr(settings, setting)}, sc.OK_200
        return {'value': "unknown setting"}, sc.NOT_FOUND_404

    async def tag_autbot_guilds(self, guild_list, user_id: int):
        try:
            all_guilds = [guild for guild in await self.bot.manager_client.all_guilds(message.AllGuildsRequest())]
        except Exception:
            logger.exception(f"Shard {self.bot.shard_id} failed to get guild list from manager")
            return {'guilds': []}, sc.INTERNAL_SERVER_ERROR_500
        for guild_dict in guild_list:
            for guild in all_guilds:
                if guild.id == int(guild_dict['id']):
                    guild_dict['has_architus'] = True
                    guild_dict['architus_admin'] = user_id in guild.admin_ids
                    break
            else:
                guild_dict.update({'has_architus': False, 'architus_admin': False})
        return {'guilds': guild_list}, sc.OK_200

    async def pool_request(self, user_id, guild_id, pool_type: str, entity_ids, fetch=False):
        guild = self.bot.get_guild(int(guild_id)) if guild_id else None
        resp = {'data': [], 'nonexistant': []}

        if pool_type == PoolType.MEMBER:
            tasks = {eid: create_task(self.pools.get_member(guild, eid, fetch)) for eid in entity_ids}
        elif pool_type == PoolType.USER:
            tasks = {eid: create_task(self.pools.get_user(eid, fetch)) for eid in entity_ids}
        elif pool_type == PoolType.EMOJI:
            tasks = {eid: create_task(self.pools.get_emoji(guild, eid, fetch)) for eid in entity_ids}
        elif pool_type == PoolType.GUILD:
            tasks = {eid: create_task(self.pools.get_guild(user_id, eid, fetch)) for eid in entity_ids}
        else:
            raise Exception(f"unknown pool type: {pool_type}")

        for entity_id, task in tasks.items():
            try:
                resp['data'].append(await task)
            except Exception as e:
                logger.debug(e)
                resp['nonexistant'].append(entity_id)
        return resp, sc.OK_200

    @fetch_guild
    async def pool_all_request(self, guild, pool_type: str):
        if pool_type == PoolType.MEMBER:
            # return {'message': "Invalid Request"}, sc.BAD_REQUEST_400
            return {'data': self.pools.get_all_members(guild)}, sc.OK_200
        elif pool_type == PoolType.CHANNEL:
            return {'data': self.pools.get_all_channels(guild)}, sc.OK_200
        elif pool_type == PoolType.ROLE:
            return {'data': self.pools.get_all_roles(guild)}, sc.OK_200
        elif pool_type == PoolType.USER:
            return {'message': "Invalid Request"}, sc.BAD_REQUEST_400
        elif pool_type == PoolType.EMOJI:
            return {'data': await self.pools.get_all_emoji(guild)}, sc.OK_200
        elif pool_type == PoolType.GUILD:
            return {'error': "Invalid Pool"}, sc.BAD_REQUEST_400
        elif pool_type == PoolType.AUTO_RESPONSE:
            return {'data': self.pools.get_all_responses(guild)}, sc.OK_200
        elif pool_type == PoolType.SETTING_VALUE:
            pass
        else:
            return {'error': "Unknown Pool"}, sc.BAD_REQUEST_400

    async def handle_mock_user_action(
            self,
            action: int = None,
            messageId: int = None,
            guildId: int = None,
            content: str = None,
            allowedCommands: List[str] = (),
            emoji: str = None,
            silent: bool = False):

        message_id = messageId
        guild_id = guildId
        allowed_commands = allowedCommands

        # this is very scuffed. guilds under this number won't have their responses added to the db
        assert guild_id < FAKE_GUILD_IDS

        if action is None or message_id is None or guild_id is None:
            return {'message': "missing arguments"}, sc.BAD_REQUEST_400

        sends = []
        reactions = []
        self.fake_messages.setdefault(guild_id, {})
        resp_id = secrets.randbits(24) | 1

        if action == LogActions.MESSAGE_SEND:
            args = content.split()

            # intersection of commands that exist and commands they're allowed to see
            all_allowed = ['poll', 'xpoll', 'schedule', 'set', 'remove']
            possible_commands = [cmd for cmd in self.bot.commands
                                 if cmd.name in allowed_commands and cmd.name in all_allowed]

            # check if they triggered help command
            if args[0][1:] == 'help':
                help_text = ''
                for cmd in possible_commands:
                    try:
                        if args[1] in cmd.aliases or args[1] == cmd.name:
                            help_text += f'```{args[1]} - {cmd.help}```'
                            break
                    except IndexError:
                        help_text += f'```{cmd.name}: {cmd.help:>5}```\n'

                sends.append(help_text)
            else:
                # check if they triggered a builtin command
                triggered_command = None
                for cmd in possible_commands:
                    if args[0][1:] in cmd.aliases + [cmd.name]:
                        triggered_command = cmd
                        break

                mock_message = MockMessage(self.bot, message_id, sends, reactions, guild_id, content=content,
                                           resp_id=resp_id)
                self.fake_messages[guild_id][message_id] = mock_message

                responses = self.bot.get_cog("Auto Responses").responses
                responses.setdefault(
                    guild_id, GuildAutoResponses(
                        self.bot, MockGuild(guild_id), None, no_db=int(guild_id) < FAKE_GUILD_IDS))
                if triggered_command:
                    # found builtin command, creating fake context
                    ctx = Context(**{
                        'message': mock_message,
                        'bot': self.bot,
                        'args': args[1:],
                        'prefix': content[0],
                        'command': triggered_command,
                        'invoked_with': args[0]
                    })

                    # override send, so ctx sends go to our list

                    async def ctx_send(content):
                        sends.append(content)
                    ctx.send = ctx_send
                    try:
                        await ctx.invoke(triggered_command, *args[1:])
                    except TypeError:
                        await ctx.invoke(triggered_command)
                else:
                    # no builtin, check for user set commands in this "guild"
                    for resp in responses[guild_id].auto_responses:
                        resp_msg, r = await responses[guild_id].execute(mock_message)
                        if r is not None:
                            break

            # Prevent response sending for silent requests
            if silent or not sends:
                sends = ()
                resp_id = None
            else:
                mock_message = MockMessage(self.bot, resp_id, sends, reactions, guild_id, content='\n'.join(sends))
                self.fake_messages[guild_id][resp_id] = mock_message

            resp = {
                'guildId': guild_id,
                'actions': [{
                    'action': LogActions.MESSAGE_SEND,
                    'content': '\n'.join(sends),
                    'messageId': resp_id,
                }]
            }
            resp['actions'] += [{
                'action': LogActions.REACTION_ADD,
                'emoji': r[1],
                'messageId': resp_id,
            } for r in reactions]

        elif action == LogActions.MESSAGE_DELETE:
            pass

        elif action == LogActions.REACTION_ADD:
            resp_id = message_id
            fkmsg = self.fake_messages[guild_id][resp_id]
            fkmsg.sends = sends
            react = await fkmsg.add_reaction(emoji, bot=False)
            await self.bot.cogs["Events"].on_reaction_add(react, MockMember())

            resp = {
                'guildId': guild_id,
                'actions': ({
                    'action': LogActions.MESSAGE_EDIT,
                    'content': '\n'.join(sends),
                    'messageId': resp_id,
                },)
            }
        elif action == LogActions.REACTION_REMOVE:
            resp_id = message_id
            fkmsg = self.fake_messages[guild_id][resp_id]
            fkmsg.sends = [fkmsg.content]
            react = await fkmsg.remove_reaction(emoji)
            await self.bot.cogs["Events"].on_reaction_remove(react, MockMember())

            resp = {
                'guildId': guild_id,
                'actions': ({
                    'action': LogActions.MESSAGE_EDIT,
                    'content': '\n'.join(sends),
                    'messageId': resp_id,
                },)
            }

        return resp, sc.OK_200


def setup(bot):
    bot.add_cog(Api(bot))
