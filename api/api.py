from flask import Flask, redirect, request
from flask_restful import Api, Resource, reqparse
from flask_cors import CORS
import requests
import json
import zmq
import os
import secrets
from datetime import datetime, timedelta

from src.config import client_id, client_secret, get_session
from src.models import AppSession, Command

API_ENDPOINT = 'https://discordapp.com/api/v6'
# REDIRECT_URI = 'https://aut-bot.com/app'
REDIRECT_URI = 'https://api.archit.us/redirect'
# REDIRECT_URI = 'http://localhost:5000/home'

app = Flask(__name__)
cors = CORS(app)


@app.route('/issue')
def issue():
    return redirect('https://github.com/aut-bot-com/autbot/issues/new')


def authenticate(session, headers):
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
    def __init__(self, q=None):
        self.session = get_session(os.getpid())
        self.q = q
        ctx = zmq.Context()
        self.topic = str(os.getpid())
        self.sub = ctx.socket(zmq.SUB)
        self.sub.connect("tcp://127.0.0.1:7200")
        self.sub.setsockopt(zmq.SUBSCRIBE, self.topic.encode())

    def enqueue(self, call):
        call['topic'] = self.topic
        self.q.put(json.dumps(call))

    def recv(self):
        return json.loads(self.sub.recv().decode().replace(self.topic + ' ', ''))


class Login(CustomResource):
    def get(self):
        nonce = str(secrets.randbits(24))
        # redirects[nonce] = request.args.get('return') or 'https://aut-bot.com/app'
        self.enqueue(
            {'method': "store_callback", 'args': [nonce, request.args.get('return') or 'https://archit.us/app']})
        self.recv()
        response = redirect('https://discordapp.com/api/oauth2/authorize?client_id=448546825532866560&redirect_uri='
                            'https%3A%2F%2Fapi.archit.us%2Fredirect&response_type=code&scope=identify%20guilds')
        response.set_cookie('redirect-nonce', nonce)
        return response


class Invite(CustomResource):
    def get(self, guild_id):
        nonce = str(secrets.randbits(24))
        self.enqueue(
            {'method': "store_callback", 'args': [nonce, request.args.get('return') or 'https://archit.us/app']})
        self.recv()
        response = redirect(f'https://discordapp.com/oauth2/authorize?client_id={client_id}'
                            '&scope=bot&guild_id={guild_id}'
                            '&response_type=code'
                            '&redirect_uri=https://api.archit.us/redirect'
                            '&permissions=2134207679')
        response.set_cookie('redirect-nonce', nonce)
        return response


class RedirectCallback(CustomResource):
    '''
    Hit by discord returning from auth.
    collects the return url from the cookie and sends the user back where they came from
    '''
    def get(self):
        self.enqueue({'method': "get_callback", 'args': [request.cookies.get('redirect-nonce')]})
        redirect_url = self.recv()['content']
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
        self.enqueue({'method': "fetch_user_dict", 'args': [name]})
        name = self.recv()
        return name, 200

    def post(self, name):
        return "not implemented", 418


class GuildCounter(CustomResource):
    def get(self):
        self.enqueue({'method': "guild_counter", 'args': []})
        return self.recv(), 200


class AutoResponses(CustomResource):
    def get(self, guild_id):
        # TODO this should probably be authenticated
        if authenticate(self.session, request.headers) is None and False:
            return "not authorized", 401
        rows = self.session.query(Command).filter(Command.trigger.startswith(str(guild_id))).all()
        commands = []
        authors = {}
        for cmd in rows:
            commands.append({
                'trigger': cmd.trigger.replace(str(cmd.server_id), "", 1),
                'response': cmd.response,
                'count': cmd.count,
                'author_id': cmd.author_id
            })
            if cmd.author_id not in authors:
                self.enqueue({'method': "fetch_user_dict", 'args': [cmd.author_id]})
                authors[cmd.author_id] = self.recv()

        resp = {
            'authors': authors,
            'commands': commands
        }
        return resp, 200

    def post(self, guild_id):
        row = authenticate(self.session, request.headers)
        if row is None:
            return "not authorized", 401
        user_id = row.discord_id

        parser = reqparse.RequestParser()
        parser.add_argument('trigger')
        parser.add_argument('response')
        args = parser.parse_args()
        if args.get('trigger') is None or args.get('response') is None:
            return "Malformed request", 400

        self.enqueue({'method': "set_response", 'args': [user_id, guild_id, args.get('trigger'), args.get('response')]})
        return self.recv(), 200


class Settings(CustomResource):
    def get(self, guild_id, setting):
        # discord_id = authenticate(self.session, request.headers).discord_id
        self.enqueue({'method': "settings_access", 'args': [guild_id, setting, None]})
        return self.recv(), 200

    def post(self, guild_id, setting):
        # parser = reqparse.RequestParser()
        # parser.add_argument('value')
        # args = parser.parse_args()
        return 400


class Coggers(CustomResource):
    '''provide an endpoint to reload cogs in the bot'''
    def get(self, extension):
        discord_id = authenticate(self.session, request.headers).discord_id
        if discord_id and discord_id == 214037134477230080:
            self.enqueue({'method': "reload_extension", 'args': [extension]})
            self.recv()
            return {}, 204
        return {"message": "401: not johnyburd"}, 401


class Identify(Resource):

    def get(self):
        session = get_session(os.getpid())
        headers = request.headers
        print(headers)
        discord_token = authenticate(session, headers).discord_access_token
        if discord_token:
            return discord_identify_request(discord_token)

        return "token invalid or expired", 401


class ListGuilds(CustomResource):

    def get(self):
        session = get_session(os.getpid())
        headers = request.headers
        print(headers)
        row = authenticate(session, headers)
        discord_token = row.discord_access_token
        if discord_token:
            headers = {
                'Content-Type': 'application/x-www-form-urlencoded',
                'Authorization': f"Bearer {discord_token}"
            }
            r = requests.get('%s/users/@me/guilds' % API_ENDPOINT, headers=headers)
            if r.status_code == 200:
                self.enqueue({'method': "tag_autbot_guilds", 'args': [r.json(), row.discord_id]})
                resp = self.recv()
            else:
                resp = r.json()
            return resp, r.status_code

        return "token invalid or expired", 401


def commit_tokens(autbot_token, discord_token, refresh_token, expires_in, discord_id):
    session = get_session(os.getpid())
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
    if r.status_code == 200:
        print(resp_data)

        discord_token = resp_data['access_token']
        autbot_token = secrets.token_urlsafe()
        expires_in = resp_data['expires_in']
        refresh_token = resp_data['refresh_token']

        resp_data, status_code = discord_identify_request(discord_token)
        if status_code == 200:
            print(resp_data)
            commit_tokens(autbot_token, discord_token, refresh_token, expires_in, resp_data['id'])
            return json.dumps({
                'access_token': autbot_token,
                'expires_in': expires_in,
                'username': resp_data['username'],
                'discriminator': resp_data['discriminator'],
                'avatar': resp_data['avatar'],
                'id': resp_data['id']
            }), 200
    return json.dumps(resp_data), r.status_code


@app.route('/status', methods=['GET'])
def status():
    return "all systems operational", 204


def app_factory(q):
    api = Api(app)
    api.add_resource(User, "/user/<string:name>", resource_class_kwargs={'q': q})
    api.add_resource(Settings, "/settings/<int:guild_id>/<string:setting>", resource_class_kwargs={'q': q})
    api.add_resource(Identify, "/identify")
    api.add_resource(ListGuilds, "/guilds", resource_class_kwargs={'q': q})
    api.add_resource(Login, "/login", resource_class_kwargs={'q': q})
    api.add_resource(AutoResponses, "/responses/<int:guild_id>", resource_class_kwargs={'q': q})
    api.add_resource(RedirectCallback, "/redirect", resource_class_kwargs={'q': q})
    api.add_resource(GuildCounter, "/guild_count", resource_class_kwargs={'q': q})
    api.add_resource(Invite, "/invite/<string:guild_id>")
    api.add_resource(Coggers, "/coggers/<string:extension>", resource_class_kwargs={'q': q})
    return app
