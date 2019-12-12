import json
import re

from flask import Flask, redirect, request, g, jsonify
from flask_restful import Api, Resource
from flask_cors import CORS

from lib.status_codes import StatusCodes
from lib.config import client_id, domain_name as DOMAIN, REDIRECT_URI
from lib.models import Command, Log
from lib.auth import JWT, flask_authenticated as authenticated

from src.discord_requests import list_guilds_request
from src.util import CustomResource, reqparams
from src.session import Identify, Login, RefreshToken, TokenExchange


app = Flask(__name__)
cors = CORS(app)


@app.teardown_appcontext
def teardown_db(arg):
    db = g.pop('db', None)
    if db is not None:
        db.close()


class Invite(Resource):
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
        return self.shard.fetch_user_dict(name)


class GuildCounter(CustomResource):
    def get(self):
        return self.shard.guild_count()


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
                emojis[str(match.group("emoji_id"))], sc = self.shard.get_emoji(match.group('emoji_id'))
            if str(cmd.author_id) not in authors:
                authors[str(cmd.author_id)], sc = self.shard.fetch_user_dict(cmd.author_id)

        resp = {
            'authors': authors,
            'emojis': emojis,
            'commands': commands
        }
        return resp, StatusCodes.OK_200

    @reqparams(trigger=str, response=str)
    @authenticated
    def post(self, guild_id: int, trigger: str, response: str, jwt: JWT):
        return self.shard.set_response(jwt.id, guild_id, trigger, response, routing_guild=guild_id)

    @reqparams(trigger=str)
    @authenticated
    def delete(self, guild_id: int, trigger: str, jwt: JWT):
        return self.shard.delete_response(jwt.id, guild_id, trigger, routing_guild=guild_id)

    @reqparams(trigger=str, response=str)
    @authenticated
    def patch(self, guild_id: int, trigger: str, response: str, jwt: JWT):
        _, sc = self.shard.delete_response(jwt.id, guild_id, trigger, routing_guild=guild_id)

        return self.shard.set_response(jwt.id, guild_id, trigger, response, routing_guild=guild_id)


class Settings(CustomResource):
    def get(self, guild_id: int, setting: str = None):
        if setting is None:
            with open('settings.json') as f:
                return json.loads(f.read()), 200
        # discord_id = authenticate(self.session, request.headers).discord_id
        return self.shard.settings_access(guild_id, setting, None, routing_guild=guild_id)

    def post(self, guild_id: int, setting: str):
        return StatusCodes.BAD_REQUEST_400


class Coggers(CustomResource):
    '''provide an endpoint to reload cogs in the bot'''
    @authenticated
    def get(self, jwt: JWT = None, extension: str = None):
        if jwt.id == 214037134477230080:  # johnyburd
            return self.shard.get_extensions()
        return {"message": "401: not johnyburd"}, StatusCodes.UNAUTHORIZED_401

    @authenticated
    def post(self, extension: str, jwt: JWT):
        if jwt.id == 214037134477230080:  # johnyburd
            return self.shard.reload_extension(extension)
        return {"message": "401: not johnyburd"}, StatusCodes.UNAUTHORIZED_401


class Stats(CustomResource):
    def get(self, guild_id: int, stat: str):
        '''Request message count statistics from shard and return'''
        if stat == 'messagecount':
            return self.shard.messagecount(guild_id, routing_guild=guild_id)


class ListGuilds(CustomResource):
    @authenticated
    def get(self, jwt: JWT):
        '''Forward guild list request to discord and return response'''
        resp, status_code = list_guilds_request()
        if status_code == StatusCodes.OK_200:
            resp, _ = self.shard.tag_autbot_guilds(resp, jwt.id)
        return resp, status_code


@app.route('/status')
def status():
    return "all systems operational", StatusCodes.NO_CONTENT_204


def app_factory():
    api = Api(app)
    api.add_resource(Identify, "/session/identify")
    api.add_resource(Login, "/session/login")
    api.add_resource(RefreshToken, "/session/refresh")
    api.add_resource(TokenExchange, "/session/token-exchange")

    api.add_resource(User, "/user/<string:name>")
    api.add_resource(Settings, "/settings/<int:guild_id>/<string:setting>", "/settings/<int:guild_id>")
    api.add_resource(ListGuilds, "/guilds")
    api.add_resource(Stats, "/stats/<int:guild_id>/<string:stat>")
    api.add_resource(AutoResponses, "/responses/<int:guild_id>")
    api.add_resource(Logs, "/logs/<int:guild_id>")
    api.add_resource(RedirectCallback, "/redirect")
    api.add_resource(GuildCounter, "/guild-count")
    api.add_resource(Invite, "/invite/<int:guild_id>")
    api.add_resource(Coggers, "/coggers/<string:extension>", "/coggers")
    return app
