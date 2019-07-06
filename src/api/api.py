import json
import traceback
import asyncio
import secrets
import websockets
import re
from discord.ext.commands import Cog, Context
from src.user_command import UserCommand, VaguePatternError, LongResponseException, ShortTriggerException, ResponseKeywordException, DuplicatedTriggerException, update_command

CALLBACK_URL = "https://archit.us/app"


class Api(Cog):

    def __init__(self, bot):
        self.bot = bot
        self.fake_messages = {}
        self.callback_urls = {}

    async def handle_socket(self, websocket, path):
        while True:
            try:
                try:
                    data = json.loads(await websocket.recv())
                    print("recv: " + str(data))
                    if data['_module'] == 'interpret':
                        resp = await self.interpret(**data)
                    else:
                        resp = {'content': "Unknown module"}
                except Exception as e:
                    traceback.print_exc()
                    print(f"caught {e} while handling websocket request")
                    resp = {'content': f"caught {e} while handling websocket request"}
                await websocket.send(json.dumps(resp))
            except websockets.exceptions.ConnectionClosed:
                print("Websocket connection closed")
                return

    @asyncio.coroutine
    def handle_request(self, pub, msg):
        try:
            resp = json.dumps((yield from getattr(self, msg['method'])(*msg['args'])))
        except Exception as e:
            traceback.print_exc()
            print(f"caught {e} while handling {msg['topic']}s request")
            resp = '{"message": "' + str(e) + '"}'
        yield from pub.send((str(msg['topic']) + ' ' + str(resp)).encode())

    async def store_callback(self, nonce=None, url=None):
        assert nonce and url
        if not any(re.match(pattern, url) for pattern in (
                r'https:\/\/[A-Fa-f0-9]{24}--architus\.netlify\.com\/app',
                r'https:\/\/deploy-preview-[0-9]+--architus\.netlify\.com\/app',
                r'https:\/\/develop\.archit\.us\/app',
                r'https:\/\/archit\.us\/app',
                r'http:\/\/localhost:3000\/app')):
            url = CALLBACK_URL

        self.callback_urls[nonce] = url
        return {"content": True}

    async def get_callback(self, nonce=None):
        url = self.callback_urls.pop(nonce, CALLBACK_URL)
        resp = {"content": url}
        return resp

    async def guild_counter(self):
        return {'guild_count': self.bot.guild_counter[0], 'user_count': self.bot.guild_counter[1]}

    async def set_response(self, user_id, guild_id, trigger, response):
        guild = self.bot.get_guild(int(guild_id))
        try:
            command = UserCommand(self.bot.session, trigger, response, 0, guild, user_id, new=True)
        except VaguePatternError:
            msg = "Capture group too broad."
        except LongResponseException:
            msg = "Response is too long."
        except ShortTriggerException as e:
            msg = "Trigger is too short."
        except ResponseKeywordException:
            msg = "That response is protected, please use another."
        except DuplicatedTriggerException as e:
            msg = "Remove duplicated trigger first."
        else:
            self.bot.user_commands[guild_id].append(command)
            msg = 'Sucessfully Set'
        return {'message': msg}

    async def fetch_user_dict(self, id):
        usr = self.bot.get_user(int(id))
        if usr is None:
            return None
        return {
            'name': usr.name,
            'avatar': usr.avatar,
            'discriminator': usr.discriminator
        }

    async def reload_extension(self, extension_name):
        name = extension_name.replace('-', '.')
        print(f"reloading extention: {name}")
        self.bot.reload_extension(name)
        return {}

    async def settings_access(self, guild_id=None, setting=None, value=None):
        guild_settings = self.bot.get_cog("GuildSettings")
        guild = self.bot.get_guild(guild_id)
        settings = guild_settings.get_guild(guild, self.bot.session)
        if hasattr(settings, setting):
            return {'value': getattr(settings, setting)}
        return {'value': "unknown setting"}

    async def tag_autbot_guilds(self, guild_list, user_id):
        guild_settings = self.bot.get_cog("GuildSettings")
        for guild_dict in guild_list:
            guild = self.bot.get_guild(int(guild_dict['id']))
            settings = guild_settings.get_guild(guild, self.bot.session)
            guild_dict['has_architus'] = guild is not None
            guild_dict['architus_admin'] = bool(settings) and user_id in settings.admins_ids
        return guild_list

    async def interpret(
            self,
            guild_id=None,
            content=None,
            message_id=None,
            added_reactions=(),
            removed_reactions=(),
            allowed_commands=(),
            silent=False,
            **k):
        sends = []
        reactions = []
        edit = False
        self.fake_messages.setdefault(guild_id, {})
        resp_id = secrets.randbits(24) | 1

        if content:
            # search for builtin commands
            command = None
            args = content.split()
            possible_commands = [cmd for cmd in self.bot.commands if cmd.name in allowed_commands]
            for cmd in possible_commands:
                if args[0][1:] in cmd.aliases + [cmd.name]:
                    command = cmd
                    break

            mock_message = MockMessage(self.bot, message_id, sends, reactions, guild_id, content=content,
                                       resp_id=resp_id)
            self.fake_messages[guild_id][message_id] = mock_message

            self.bot.user_commands.setdefault(int(guild_id), [])
            if command:
                # found builtin command, creating fake context
                ctx = Context(**{
                    'message': mock_message,
                    'bot': self.bot,
                    'args': args[1:],
                    'prefix': content[0],
                    'command': command,
                    'invoked_with': args[0]
                })
                ctx.send = lambda content: sends.append(content)
                await ctx.invoke(command, *args[1:])
            elif args[0][1:] == 'help':
                help_text = ''
                for cmd in possible_commands:
                    try:
                        if args[1] in cmd.aliases or args[1] == cmd.name:
                            help_text += f'```{args[1]} - {cmd.help}```'
                            break
                    except IndexError:
                        help_text += '```{}: {:>5}```\n'.format(cmd.name, cmd.help)

                sends.append(help_text)
            else:
                # check for user set commands in this "guild"
                for command in self.bot.user_commands[mock_message.guild.id]:
                    if (command.triggered(mock_message.content)):
                        await command.execute(mock_message, self.bot.session)
                        break

            # Prevent response sending for silent requests
            if silent or not sends:
                sends = ()
                resp_id = None
            else:
                mock_message = MockMessage(self.bot, resp_id, sends, reactions, guild_id, content='\n'.join(sends))
                self.fake_messages[guild_id][resp_id] = mock_message

        elif added_reactions:
            edit = True
            resp_id = added_reactions[0][0]
            for react in added_reactions:
                fkmsg = self.fake_messages[guild_id][react[0]]
                fkmsg.sends = sends
                react = await fkmsg.add_reaction(react[1], bot=False)
                await self.bot.get_cog("EventCog").on_reaction_add(react, MockMember())
        elif removed_reactions:
            edit = True
            resp_id = removed_reactions[0][0]
            for react in removed_reactions:
                fkmsg = self.fake_messages[guild_id][react[0]]
                fkmsg.sends = sends
                react = await fkmsg.remove_reaction(react[1])
                await self.bot.get_cog("Events").on_reaction_remove(react, MockMember())
        resp = {
            '_module': 'interpret',
            'content': '\n'.join(sends),
            'added_reactions': [(r[0], r[1]) for r in reactions],
            'message_id': resp_id,
            'edit': edit,
            'guild_id': guild_id,
        }
        if resp['content']:
            print(resp)
        return resp


class MockMember(object):
    def __init__(self, id=0):
        self.id = id
        self.mention = "<@%_CLIENT_ID_%>"
        self.display_name = "bad guy"
        self.bot = False


class MockRole(object):
    pass


class MockChannel(object):
    def __init__(self, bot, sends, reactions, resp_id):
        self.bot = bot
        self.sends = sends
        self.reactions = reactions
        self.resp_id = resp_id

    async def send(self, *args):
        for thing in args:
            self.sends.append(thing)
        return MockMessage(self.bot, self.resp_id, self.sends, self.reactions, 0)


class MockGuild(object):
    def __init__(self, id):
        self.region = 'us-east'
        self.id = int(id)
        self.owner = MockMember()
        self.me = MockMember()
        self.default_role = MockRole()
        self.default_role.mention = "@everyone"
        self.emojis = []

    def get_member(self, *args):
        return None


class MockReact(object):
    def __init__(self, message, emoji, user):
        self.message = message
        self.emoji = emoji
        self.count = 1
        self._users = [user]

    def users(self):
        class user:
            pass
        u = user()

        async def flatten():
            return self._users
        u.flatten = flatten
        return u


class MockMessage(object):
    def __init__(self, bot, id, sends, reaction_sends, guild_id, content=None, resp_id=0):
        self.bot = bot
        self.id = id
        self.sends = sends
        self.reaction_sends = reaction_sends
        self._state = MockChannel(bot, sends, reaction_sends, resp_id)
        self.guild = MockGuild(guild_id)
        self.author = MockMember()
        self.channel = MockChannel(bot, sends, reaction_sends, resp_id)
        self.content = content
        self.reactions = []

    async def add_reaction(self, emoji, bot=True):
        user = MockMember()
        if bot:
            self.reaction_sends.append((self.id, emoji))
            user = self.bot
        for react in self.reactions:
            if emoji == react.emoji:
                react._users.append(user)
                return react
        else:
            react = MockReact(self, emoji, user)
            self.reactions.append(react)
            return react

    async def remove_reaction(self, emoji):
        for react in self.reactions:
            if emoji == react.emoji:
                react._users = [self.bot.user]
                return react

    async def edit(self, content=None):
        print("EDIT " + content)
        self.sends.append(content)


def setup(bot):
    bot.add_cog(Api(bot))
