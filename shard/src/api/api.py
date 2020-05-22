import secrets
from datetime import timedelta
from typing import List

from discord.ext.commands import Cog, Context
import discord

from lib.status_codes import StatusCodes as sc
from lib.pool_types import PoolType
from lib.config import logger
from src.api.util import fetch_guild
from src.api.pools import Pools
from src.api.mock_discord import MockMember, MockMessage, LogActions


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
        return await self.bot.manager_client.guild_count()

    async def set_response(self, user_id, guild_id, trigger, response):
        return {'message': 'unimplemented'}, 500

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
    async def bin_messages(self, guild):
        stats_cog = self.bot.cogs["Server Statistics"]
        members, channels, times = stats_cog.bin_messages(guild, timedelta(minutes=5))
        return {
            'total': len(stats_cog.cache[guild.id]),
            'members': members,
            'channels': channels,
            'times': times,
        }, sc.OK_200

    @fetch_guild
    async def get_guild_data(self, guild):
        return {
            'name': guild.name,
            'member_count': guild.member_count,
        }, sc.OK_200

    @fetch_guild
    async def settings_access(self, guild, setting=None, value=None):
        settings = self.bot.settings[guild]
        if hasattr(settings, setting):
            return {'value': getattr(settings, setting)}, sc.OK_200
        return {'value': "unknown setting"}, sc.NOT_FOUND_404

    async def tag_autbot_guilds(self, guild_list, user_id: int):
        all_guilds, _ = await self.bot.manager_client.all_guilds()
        for guild_dict in guild_list:
            for guild in all_guilds:
                if int(guild['id']) == int(guild_dict['id']):
                    guild_dict['has_architus'] = True
                    guild_dict['architus_admin'] = user_id in guild['admin_ids']
                    break
            else:
                guild_dict.update({'has_architus': False, 'architus_admin': False})
        return {'guilds': guild_list}, sc.OK_200

    async def pool_request(self, guild_id, pool_type: str, entity_id, fetch=False):
        guild = self.bot.get_guild(int(guild_id)) if guild_id else None
        try:
            if pool_type == PoolType.MEMBER:
                return {'data': await self.pools.get_member(guild, entity_id, fetch)}, 200
            elif pool_type == PoolType.USER:
                return {'data': await self.pools.get_user(entity_id, fetch)}, 200
        except Exception:
            logger.exception('')
            return {'data': {}}, sc.NOT_FOUND_404

    @fetch_guild
    async def pool_all_request(self, guild, pool_type: str):
        if pool_type == PoolType.MEMBER:
            # return {'message': "Invalid Request"}, sc.BAD_REQUEST_400
            return {'data': self.pools.get_all_members(guild)}, 200
        elif pool_type == PoolType.CHANNEL:
            return {'data': self.pools.get_all_channels(guild)}, 200
        elif pool_type == PoolType.ROLE:
            return {'data': self.pools.get_all_roles(guild)}, 200
        elif pool_type == PoolType.USER:
            return {'message': "Invalid Request"}, sc.BAD_REQUEST_400
        elif pool_type == PoolType.EMOJI:
            return {'data': self.pools.get_all_emoji(guild)}, 200
        elif pool_type == PoolType.GUILD:
            return {'error': "Invalid Pool"}, sc.BAD_REQUEST_400
        elif pool_type == PoolType.AUTO_RESPONSE:
            return {'data': self.pools.get_all_responses(guild)}, 200
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
        assert guild_id < 10000000

        if action is None or message_id is None or guild_id is None:
            return {'message': "missing arguments"}, sc.BAD_REQUEST_400

        sends = []
        reactions = []
        self.fake_messages.setdefault(guild_id, {})
        resp_id = secrets.randbits(24) | 1

        if action == LogActions.MESSAGE_SEND:
            args = content.split()

            # intersection of commands that exist and commands they're allowed to see
            possible_commands = [cmd for cmd in self.bot.commands if cmd.name in allowed_commands]

            # check if they triggered help command
            if args[0][1:] == 'help':
                help_text = ''
                for cmd in possible_commands:
                    try:
                        if args[1] in cmd.aliases or args[1] == cmd.name:
                            help_text += f'```hi{args[1]} - {cmd.help}```'
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

                # self.bot.user_commands.setdefault(int(guild_id), [])
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
                    ctx.send = lambda content: sends.append(content)
                    # await ctx.invoke(triggered_command, *args[1:])
                else:
                    # no builtin, check for user set commands in this "guild"
                    for command in ():
                        if command.triggered(mock_message.content):
                            await command.execute(mock_message)
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
