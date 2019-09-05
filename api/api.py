from flask import Flask, redirect, request, g
from flask_restful import Api, Resource, reqparse
from flask_cors import CORS
import requests
import json
import os
import re
import random
from uuid import getnode

from lib.status_codes import StatusCodes
from lib.config import client_id, get_session, NUM_SHARDS, which_shard
from lib.blocking_rpc_client import get_rpc_client
from lib.models import Command, Log
from lib.auth import JWT, discord_identify_request, token_exchange_request, flask_authenticated as authenticated

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
        return self.client.call(
            method,
            *args,
            routing_key=f"shard_rpc_{which_shard(routing_guild)}",
            **kwargs
        )


class Login(CustomResource):
    def get(self):
        # TODO don't redirect to the staging api on production
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
    def post(self, guild_id, jwt):
        parser = reqparse.RequestParser()
        parser.add_argument('trigger')
        parser.add_argument('response')
        args = parser.parse_args()
        if args.get('trigger') is None or args.get('response') is None:
            return "Malformed request", StatusCodes.BAD_REQUEST_400

        return self.shard_call(
            'set_response',
            jwt.id,
            guild_id,
            args.get('trigger'),
            args.get('response'),
            routing_guild=guild_id
        )

    @authenticated
    def delete(self, guild_id, jwt):
        parser = reqparse.RequestParser()
        parser.add_argument('trigger')
        args = parser.parse_args()
        if args.get('trigger') is None:
            return "Malformed request", StatusCodes.BAD_REQUEST_400

        return self.shard_call('delete_response', jwt.id, guild_id, args.get('trigger'), routing_guild=guild_id)

    @authenticated
    def patch(self, guild_id, jwt):
        parser = reqparse.RequestParser()
        parser.add_argument('trigger')
        parser.add_argument('response')
        args = parser.parse_args()
        if args.get('trigger') is None or args.get('response') is None:
            return "Malformed request", 400

        _, sc = self.shard_call(
            'delete_response',
            jwt.id,
            guild_id,
            args.get('trigger'),
            routing_guild=guild_id
        )

        return self.shard_call(
            'set_response',
            jwt.id,
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
    def get(self, jwt=None, extension=None):
        if jwt.id == 214037134477230080:
            return self.shard_call('get_extensions')
        return {"message": "401: not johnyburd"}, StatusCodes.UNAUTHORIZED_401

    @authenticated
    def post(self, extension, jwt):
        if jwt.id == 214037134477230080:
            return self.shard_call('reload_extension', extension)
        return {"message": "401: not johnyburd"}, StatusCodes.UNAUTHORIZED_401


class Identify(Resource):
    @authenticated
    def get(self, jwt):
        return discord_identify_request(jwt.access_token)


class Stats(CustomResource):
    def get(self, guild_id, stat):
        if stat == 'messagecount':
            return self.shard_call('messagecount', guild_id, routing_guild=guild_id)


class ListGuilds(CustomResource):

    @authenticated
    def get(self, jwt):
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


@app.route('/token_exchange', methods=['POST'])
def token_exchange():
    parser = reqparse.RequestParser()
    parser.add_argument('code')
    args = parser.parse_args()
    ex_data, status_code = token_exchange_request(args['code'])
    discord_token = ex_data['access_token']

    if status_code == StatusCodes.OK_200:
        id_data, status_code = discord_identify_request(discord_token)
        if status_code == StatusCodes.OK_200:
            data = {
                'access_token': discord_token,
                'expires_in': ex_data['expires_in'],
                'refresh_token': ex_data['refresh_token'],
                'username': id_data['username'],
                'discriminator': id_data['discriminator'],
                'avatar': id_data['avatar'],
                'id': id_data['id'],
            }
            jwt = JWT(data.copy())
            data.update({'access_token': jwt.get_token()})
            return json.dumps(data), StatusCodes.OK_200

    return json.dumps(ex_data), status_code


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
