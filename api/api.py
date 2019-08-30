from flask import Flask, redirect, request, g
from flask_restful import Api, Resource, reqparse
from flask_cors import CORS
import requests
import json
import os
import re
import secrets
import random
from datetime import datetime, timedelta
from uuid import getnode

from lib.status_codes import StatusCodes
from lib.config import client_id, client_secret, get_session, NUM_SHARDS
from lib.blocking_rpc_client import get_rpc_client
from lib.models import AppSession, Command, Log

API_ENDPOINT = 'https://discordapp.com/api/v6'
# REDIRECT_URI = 'https://api.archit.us/redirect'
REDIRECT_URI = 'https://api.archit.us:8000/redirect'


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


@app.route('/issue')
def issue():
    return redirect('https://github.com/architus/architus/issues/new')


def authenticated(func):
    """decorator for rest endpoint functions
    returns 401 if user is not logged in
    and prepends their discord id to the arguments if identification is necessary
    """
    def authed_func(self, *args, **kwargs):
        row = authenticate(get_db(), request.headers)
        if row is None:
            return (StatusCodes.UNAUTHORIZED_401, "Not Authorized")
        self.discord_token = row.discord_token
        return func(self, row.discord_id, *args, **kwargs)
    return authed_func


def authenticate(session, headers):
    """lookup a token in the database"""
    try:
        autbot_token = headers['Authorization']
    except KeyError:
        return None
    rows = session.query(AppSession).filter_by(autbot_access_token=autbot_token).all()
    for row in rows:
        if datetime.now() < row.autbot_expiration:
            return row
    return None


class CustomResource(Resource):
    """Default flask Resource but contains tools to talk to the shard nodes and the db"""
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

        if routing_guild is not None:
            shard_id = (routing_guild >> 22) % NUM_SHARDS
        else:
            shard_id = random.randint(0, NUM_SHARDS - 1)

        return self.client.call(
            method,
            *args,
            routing_key=f"shard_rpc_{shard_id}",
            **kwargs
        )


class Login(CustomResource):
    def get(self):
        response = redirect(f'https://discordapp.com/api/oauth2/authorize?client_id={client_id}&redirect_uri='
                            'https%3A%2F%2Fapi.archit.us%3A8000%2Fredirect&response_type=code&scope=identify%20guilds')
        # TODO nice validation
        # if not any(re.match(pattern, url) for pattern in (
        #         r'https:\/\/[-A-Za-z0-9]{24}--architus\.netlify\.com\/app',
        #         r'https:\/\/deploy-preview-[0-9]+--architus\.netlify\.com\/app',
        #         r'https:\/\/develop\.archit\.us\/app',
        #         r'https:\/\/archit\.us\/app',
        #         r'http:\/\/localhost:3000\/app')):
        #     url = CALLBACK_URL
        response.set_cookie('next', request.args.get('return'))
        return response


class Invite(CustomResource):
    def get(self, guild_id):
        response = redirect(f'https://discordapp.com/oauth2/authorize?client_id={client_id}'
                            f'&scope=bot&guild_id={guild_id}'
                            '&response_type=code'
                            '&redirect_uri=https://api.archit.us/redirect'
                            '&permissions=2134207679')
        response.set_cookie('next', request.args.get('return'))
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
    def get(self, name):
        return self.shard_call('fetch_user_dict', name)


class GuildCounter(CustomResource):
    def get(self):
        return self.shard_call('guild_counter')


class Logs(CustomResource):
    def get(self, guild_id):
        # TODO this should probably be authenticated
        if authenticate(self.session, request.headers) is None and False:
            return "not authorized", StatusCodes.UNAUTHORIZED_401
        rows = self.session.query(Log).filter(Log.guild_id == guild_id).order_by(Log.timestamp.desc()).limit(400).all()
        logs = []
        for log in rows:
            logs.append({
                'type': log.type,
                'content': log.content,
                'user_id': str(log.user_id),
                'timestamp': log.timestamp.isoformat()
            })
        return json.dumps({"logs": logs}), StatusCodes.OK_200


class AutoResponses(CustomResource):
    def get(self, guild_id):
        # TODO this should probably be authenticated
        if authenticate(self.session, request.headers) is None and False:
            return "not authorized", StatusCodes.UNAUTHORIZED_401
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

    @authenticated
    def post(self, user_id, guild_id):
        parser = reqparse.RequestParser()
        parser.add_argument('trigger')
        parser.add_argument('response')
        args = parser.parse_args()
        if args.get('trigger') is None or args.get('response') is None:
            return "Malformed request", StatusCodes.BAD_REQUEST_400

        return self.shard_call(
            'set_response',
            user_id,
            guild_id,
            args.get('trigger'),
            args.get('response'),
            routing_guild=guild_id
        )

    @authenticated
    def delete(self, user_id, guild_id):
        parser = reqparse.RequestParser()
        parser.add_argument('trigger')
        args = parser.parse_args()
        if args.get('trigger') is None:
            return "Malformed request", StatusCodes.BAD_REQUEST_400

        return self.shard_call('delete_response', user_id, guild_id, args.get('trigger'), routing_guild=guild_id)

    @authenticated
    def patch(self, user_id, guild_id):
        parser = reqparse.RequestParser()
        parser.add_argument('trigger')
        parser.add_argument('response')
        args = parser.parse_args()
        if args.get('trigger') is None or args.get('response') is None:
            return "Malformed request", 400

        _, sc = self.shard_call(
            'delete_response',
            user_id,
            guild_id,
            args.get('trigger'),
            routing_guild=guild_id
        )

        return self.shard_call(
            'set_response',
            user_id,
            guild_id,
            args.get('trigger'),
            args.get('response'),
            routing_guild=guild_id
        )


class Settings(CustomResource):
    def get(self, guild_id, setting=None):
        if setting is None:
            with open('settings.json') as f:
                return json.loads(f.read()), 200
        # discord_id = authenticate(self.session, request.headers).discord_id
        return self.shard_call('settings_access', guild_id, setting, None, routing_guild=guild_id)

    def post(self, guild_id, setting):
        # parser = reqparse.RequestParser()
        # parser.add_argument('value')
        # args = parser.parse_args()
        return StatusCodes.BAD_REQUEST_400


class Coggers(CustomResource):
    '''provide an endpoint to reload cogs in the bot'''
    @authenticated
    def get(self, user_id, extension=None):
        if user_id == 214037134477230080:
            return self.shard_call('get_extensions')
        return {"message": "401: not johnyburd"}, StatusCodes.UNAUTHORIZED_401

    @authenticated
    def post(self, user_id, extension):
        if user_id == 214037134477230080:
            return self.shard_call('reload_extension', extension)
        return {"message": "401: not johnyburd"}, StatusCodes.UNAUTHORIZED_401


class Identify(Resource):

    def get(self):
        row = authenticate(get_db(), request.headers)
        if row and row.discord_access_token:
            return discord_identify_request(row.discord_access_token)

        return "token invalid or expired", StatusCodes.UNAUTHORIZED_401


class Stats(CustomResource):
    def get(self, guild_id, stat):
        if stat == 'messagecount':
            return self.shard_call('messagecount', guild_id, routing_guild=guild_id)


class ListGuilds(CustomResource):

    def get(self):
        row = authenticate(self.session, request.headers)
        discord_token = row.discord_access_token
        if discord_token:
            headers = {
                'Content-Type': 'application/x-www-form-urlencoded',
                'Authorization': f"Bearer {discord_token}"
            }
            r = requests.get('%s/users/@me/guilds' % API_ENDPOINT, headers=headers)
            if r.status_code == StatusCodes.OK_200:
                resp, sc = self.shard_call('tag_autbot_guilds', r.json(), row.discord_id)
            else:
                resp = r.json()
            return resp, r.status_code

        return "token invalid or expired", StatusCodes.UNAUTHORIZED_401


def commit_tokens(autbot_token, discord_token, refresh_token, expires_in, discord_id):
    session = get_db()
    time = datetime.now() + timedelta(seconds=int(expires_in) - 60)
    new_appsession = AppSession(autbot_token, discord_token, refresh_token, time, time, discord_id, datetime.now())
    session.add(new_appsession)
    session.commit()


def discord_identify_request(token):
    headers = {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Authorization': f"Bearer {token}"
    }
    r = requests.get('%s/users/@me' % API_ENDPOINT, headers=headers)
    return r.json(), r.status_code


@app.route('/token_exchange', methods=['POST'])
def token_exchange():
    parser = reqparse.RequestParser()
    parser.add_argument('code')
    args = parser.parse_args()
    data = {
        'client_id': client_id,
        'client_secret': client_secret,
        'grant_type': 'authorization_code',
        'code': args['code'],
        'redirect_uri': REDIRECT_URI,
        'scope': 'identify'
    }
    headers = {
        'Content-Type': 'application/x-www-form-urlencoded'
    }
    r = requests.post('%s/oauth2/token' % API_ENDPOINT, data=data, headers=headers)
    resp_data = r.json()
    print(r.status_code)
    print(r.text)
    if r.status_code == StatusCodes.OK_200:
        print(resp_data)

        discord_token = resp_data['access_token']
        autbot_token = secrets.token_urlsafe()
        expires_in = resp_data['expires_in']
        refresh_token = resp_data['refresh_token']

        resp_data, status_code = discord_identify_request(discord_token)
        if status_code == StatusCodes.OK_200:
            print(resp_data)
            commit_tokens(autbot_token, discord_token, refresh_token, expires_in, resp_data['id'])
            return json.dumps({
                'access_token': autbot_token,
                'expires_in': expires_in,
                'username': resp_data['username'],
                'discriminator': resp_data['discriminator'],
                'avatar': resp_data['avatar'],
                'id': resp_data['id']
            }), StatusCodes.OK_200
    return json.dumps(resp_data), r.status_code


@app.route('/status', methods=['GET'])
def status():
    return "all systems operational", StatusCodes.NO_CONTENT_204


def app_factory():
    api = Api(app)
    api.add_resource(User, "/user/<string:name>")
    api.add_resource(Settings, "/settings/<int:guild_id>/<string:setting>", "/settings/<int:guild_id>")
    api.add_resource(Identify, "/identify")
    api.add_resource(ListGuilds, "/guilds")
    api.add_resource(Stats, "/stats/<int:guild_id>/<string:stat>")
    api.add_resource(Login, "/login")
    api.add_resource(AutoResponses, "/responses/<int:guild_id>")
    api.add_resource(Logs, "/logs/<int:guild_id>")
    api.add_resource(RedirectCallback, "/redirect")
    api.add_resource(GuildCounter, "/guild_count")
    api.add_resource(Invite, "/invite/<int:guild_id>")
    api.add_resource(Coggers, "/coggers/<string:extension>", "/coggers")
    return app
