from flask import Flask, redirect, request, g, jsonify, make_response
from flask_restful import Api, Resource, reqparse
from flask_cors import CORS
import requests
import json
import os
import re
from uuid import getnode
from datetime import datetime, timedelta
from urllib.parse import quote_plus

from lib.status_codes import StatusCodes
from lib.config import client_id, domain_name, get_session, which_shard
from lib.blocking_rpc_client import get_rpc_client
from lib.models import Command, Log
from lib.auth import JWT, discord_identify_request, token_exchange_request, flask_authenticated as authenticated

API_ENDPOINT = 'https://discordapp.com/api/v6'
DOMAIN = domain_name
REDIRECT_URI = f'https://api.{DOMAIN}/redirect'
SAFE_REDIRECT_URI = quote_plus(REDIRECT_URI)


app = Flask(__name__)
cors = CORS(app)


def get_db():
    if 'db' not in g:
        g.db = get_session()
    return g.db


@app.teardown_appcontext
def teardown_db(arg):
    db = g.pop('db', None)
    if db is not None:
        db.close()


def reqparams(**params):
    def decorator(func):
        def wrapper(*args, **kwargs):
            parser = reqparse.RequestParser()
            for param, type in params:
                parser.add_argument(param, type=type, required=True)
            values = parser.parse_args()
            kwargs.update(values)
            func(*args, **kwargs)
        return wrapper
    return decorator


@app.route('/issue')
def issue():
    return redirect('https://github.com/architus/architus/issues/new')


class CustomResource(Resource):
    '''Default flask Resource but contains tools to talk to the shard nodes and the db.'''
    def __init__(self):
        self._session = None
        self.topic = (getnode() << 15) | os.getpid()
        self.client = get_rpc_client(self.topic)

    @property
    def session(self):
        if self._session is None:
            self._session = get_db()
        return self._session

    def shard_call(self, method, *args, routing_guild=None, **kwargs):
        '''Queues an RPC request to a shard.'''
        return self.client.call(
            method,
            *args,
            routing_key=f"shard_rpc_{which_shard(routing_guild)}",
            **kwargs
        )


class Login(CustomResource):
    def get(self):
        response = redirect(f'https://discordapp.com/api/oauth2/authorize?client_id={client_id}&redirect_uri='
                            f'{SAFE_REDIRECT_URI}&response_type=code&scope=identify%20guilds')
# TODO nice validation
# if not any(re.match(pattern, url) for pattern in (
#         r'https:\/\/[-A-Za-z0-9]{24}--architus\.netlify\.com\/app',
#         r'https:\/\/deploy-preview-[0-9]+--architus\.netlify\.com\/app',
#         r'https:\/\/develop\.archit\.us\/app',
#         r'https:\/\/archit\.us\/app',
#         r'http:\/\/localhost:3000\/app')):
#     url = CALLBACK_URL
# TODO default destination
        response.set_cookie('next', request.args.get('return'), domain=f'api.{DOMAIN}', secure=True, httponly=True)
        return response


class RefreshToken(CustomResource):
    pass


class Invite(CustomResource):
    def get(self, guild_id: int):
        response = redirect(f'https://discordapp.com/oauth2/authorize?client_id={client_id}'
                            f'&scope=bot&guild_id={guild_id}'
                            '&response_type=code'
                            f'&redirect_uri={REDIRECT_URI}'
                            '&permissions=2134207679')
        response.set_cookie('next', request.args.get('return'), domain=f'api.{DOMAIN}', secure=True, httponly=True)
        return response


class RedirectCallback(CustomResource):
    '''
    Hit by discord returning from auth.
    collects the return url from the cookie and sends the user back where they came from
    '''
    def get(self):
        # TODO validate domain
        redirect_url = request.cookies.get('next')
        code = request.args.get('code')
        perms = request.args.get('permissions')
        guild_id = request.args.get('guild_id')

        if code and not perms:
            redirect_url += f"?code={code}"
        if perms:
            redirect_url += f"?permissions={perms}"
        if guild_id:
            redirect_url += f"?guild_id={guild_id}"

        resp = redirect(redirect_url)
        return resp


class User(CustomResource):
    def get(self, name: int):
        '''Request information about a user from a shard nope and return it.'''
        return self.shard_call('fetch_user_dict', name)


class GuildCounter(CustomResource):
    def get(self):
        return self.shard_call('guild_counter')


class Logs(CustomResource):
    def get(self, guild_id: int):
        # TODO this should probably be authenticated
        rows = self.session.query(Log).filter(Log.guild_id == guild_id).order_by(Log.timestamp.desc()).limit(400).all()
        logs = []
        for log in rows:
            logs.append({
                'type': log.type,
                'content': log.content,
                'user_id': str(log.user_id),
                'timestamp': log.timestamp.isoformat()
            })
            return jsonify({"logs": logs}), StatusCodes.OK_200


class AutoResponses(CustomResource):
    def get(self, guild_id: int):
        # TODO this should probably be authenticated
        rows = self.session.query(Command).filter(Command.trigger.startswith(str(guild_id))).all()
        commands = []
        authors = {}
        emojis = {}
        p = re.compile(r"<:\S+:(?P<emoji_id>\d{15,30})\>")
        for cmd in rows:
            commands.append({
                'trigger': cmd.trigger.replace(str(cmd.server_id), "", 1),
                'response': cmd.response,
                'count': cmd.count,
                'author_id': str(cmd.author_id)
            })
            match = p.search(cmd.response)
            if match and str(match.group("emoji_id")) not in emojis:
                emojis[str(match.group("emoji_id"))], sc = self.shard_call('get_emoji', match.group('emoji_id'))
            if str(cmd.author_id) not in authors:
                authors[str(cmd.author_id)], sc = self.shard_call('fetch_user_dict', cmd.author_id)

        resp = {
            'authors': authors,
            'emojis': emojis,
            'commands': commands
        }
        return resp, StatusCodes.OK_200

    @reqparams(trigger=str, response=str)
    @authenticated
    def post(self, guild_id: int, trigger: str, response: str, jwt: JWT):
        return self.shard_call('set_response', jwt.id, guild_id, trigger, response, routing_guild=guild_id)

    @reqparams(trigger=str)
    @authenticated
    def delete(self, guild_id: int, trigger: str, jwt: JWT):
        return self.shard_call('delete_response', jwt.id, guild_id, trigger, routing_guild=guild_id)

    @reqparams(trigger=str, response=str)
    @authenticated
    def patch(self, guild_id: int, trigger: str, response: str, jwt: JWT):
        _, sc = self.shard_call('delete_response', jwt.id, guild_id, trigger, routing_guild=guild_id)

        return self.shard_call('set_response', jwt.id, guild_id, trigger, response, routing_guild=guild_id)


class Settings(CustomResource):
    def get(self, guild_id: int, setting: str = None):
        if setting is None:
            with open('settings.json') as f:
                return json.loads(f.read()), 200
        # discord_id = authenticate(self.session, request.headers).discord_id
        return self.shard_call('settings_access', guild_id, setting, None, routing_guild=guild_id)

    def post(self, guild_id: int, setting: str):
        return StatusCodes.BAD_REQUEST_400


class Coggers(CustomResource):
    '''provide an endpoint to reload cogs in the bot'''
    @authenticated
    def get(self, jwt: JWT = None, extension: str = None):
        if jwt.id == 214037134477230080:  # johnyburd
            return self.shard_call('get_extensions')
        return {"message": "401: not johnyburd"}, StatusCodes.UNAUTHORIZED_401

    @authenticated
    def post(self, extension: str, jwt: JWT):
        if jwt.id == 214037134477230080:  # johnyburd
            return self.shard_call('reload_extension', extension)
        return {"message": "401: not johnyburd"}, StatusCodes.UNAUTHORIZED_401


class Identify(Resource):
    @authenticated
    def get(self, jwt: JWT):
        '''Forward identify request to discord and return response'''
        return discord_identify_request(jwt.access_token)


class Stats(CustomResource):
    def get(self, guild_id: int, stat: str):
        '''Request message count statistics from shard and return'''
        if stat == 'messagecount':
            return self.shard_call('messagecount', guild_id, routing_guild=guild_id)


class ListGuilds(CustomResource):

    @authenticated
    def get(self, jwt: JWT):
        '''Forward guild list request to discord and return response'''
        headers = {
            'Content-Type': 'application/x-www-form-urlencoded',
            'Authorization': f"Bearer {jwt.access_token}"
        }
        r = requests.get('%s/users/@me/guilds' % API_ENDPOINT, headers=headers)
        if r.status_code == StatusCodes.OK_200:
            resp, _ = self.shard_call('tag_autbot_guilds', r.json(), jwt.id)
        else:
            resp = r.json()
        return resp, r.status_code


@app.route('/session/token_exchange', methods=['POST'])
@reqparams(code=str)
def token_exchange(code):
    ex_data, status_code = token_exchange_request(code)

    if status_code == StatusCodes.OK_200:
        discord_token = ex_data['access_token']
        id_data, status_code = discord_identify_request(discord_token)
        if status_code == StatusCodes.OK_200:
            now = datetime.now()
            expires_in = ex_data['expires_in']
            refresh_in = timedelta(seconds=expires_in) / 2
            jwt = JWT({
                'access_token': discord_token,
                'refresh_token': ex_data['refresh_token'],
                'expires_in': expires_in,
                'issued_at': now,
                'refresh_in': refresh_in,
                'id': id_data['id'],
                'permissions': 0,
            })
            data = {
                # 'token': jwt.get_token().decode()
                'user': id_data,
                'access': {
                    'issuedAt': now,
                    'expiresIn': expires_in,
                    'refreshIn': refresh_in,
                }
            }
            print(data)

            response = make_response()
            response.set_cookie("token", jwt.get_token().decode(), domain=f'.{DOMAIN}', secure=True, httponly=True)
            response.data = jsonify(data)
            response.status_code = StatusCodes.OK_200
            return response

    return jsonify(ex_data), status_code


@app.route('/status', methods=['GET'])
def status():
    return "all systems operational", StatusCodes.NO_CONTENT_204


def app_factory():
    api = Api(app)
    api.add_resource(User, "/user/<string:name>")
    api.add_resource(Settings, "/settings/<int:guild_id>/<string:setting>", "/settings/<int:guild_id>")
    api.add_resource(ListGuilds, "/guilds")
    api.add_resource(Stats, "/stats/<int:guild_id>/<string:stat>")
    api.add_resource(Identify, "/session/identify")
    api.add_resource(Login, "/session/login")
    api.add_resource(RefreshToken, "/session/refresh")
    # /session/token_exchange
    api.add_resource(AutoResponses, "/responses/<int:guild_id>")
    api.add_resource(Logs, "/logs/<int:guild_id>")
    api.add_resource(RedirectCallback, "/redirect")
    api.add_resource(GuildCounter, "/guild_count")
    api.add_resource(Invite, "/invite/<int:guild_id>")
    api.add_resource(Coggers, "/coggers/<string:extension>", "/coggers")
    return app
