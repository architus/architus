import json
from io import BytesIO

from flask import Flask, redirect, request, g, Response, make_response
from flask_restful import Api, Resource
from flask_cors import CORS
from werkzeug.wsgi import FileWrapper

from lib.status_codes import StatusCodes
from lib.config import logger, client_id, domain_name as DOMAIN, REDIRECT_URI, is_prod, which_shard
# from lib.models import Log # , Emojis
from lib.auth import JWT, flask_authenticated as authenticated, verify_twitch_hub
from lib.discord_requests import list_guilds_request
from lib.pool_types import PoolType

from src.util import CustomResource, reqparams, camelcase_keys
from src.session import Identify, Login, RefreshToken, TokenExchange, End
from src.payments import Checkout


app = Flask(__name__)
cors = CORS(
    app,
    origins=[f"https://{DOMAIN}", f"https://api.{DOMAIN}"] if is_prod else "*",
    supports_credentials=True
)


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
        response.set_cookie('next', request.args.get('return', ''), domain=f'api.{DOMAIN}', secure=True, httponly=True)
        return response


class RedirectCallback(CustomResource):
    '''
    Hit by discord returning from auth.
    collects the return url from the cookie and sends the user back where they came from
    '''
    def get(self):
        # TODO validate domain
        redirect_url = request.cookies.get('next') or f'https://{DOMAIN}/app'
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
        resp.set_cookie('next', '', expires=0)
        return resp


class User(CustomResource):
    def get(self, name: int):
        '''Request information about a user from a shard nope and return it.'''
        return self.shard.fetch_user_dict(name)


class GuildCounter(CustomResource):
    def get(self):
        guild_count, sc = self.shard.guild_count()
        camelcase_keys(guild_count)
        return guild_count, sc


class AllGuilds(CustomResource):
    @authenticated()
    def get(self, jwt: JWT):
        if jwt.id != 214037134477230080:  # johnyburd
            return {"message": "unauthorized"}, StatusCodes.UNAUTHORIZED_401
        return self.shard.all_guilds()


class Logs(CustomResource):
    @authenticated(member=True)
    def get(self, guild_id: int):
        # rows = self.session.query(Log).filter(Log.guild_id == guild_id)
        # .order_by(Log.timestamp.desc()).limit(400).all()
        rows = []
        self.session.commit()
        logs = []
        for log in rows:
            logs.append({
                'type': log.type,
                'content': log.content,
                'user_id': str(log.user_id),
                'timestamp': log.timestamp.isoformat()
            })
            return {"logs": logs}, StatusCodes.OK_200


class AutoResponses(CustomResource):
    @authenticated(member=True)
    def get(self, guild_id: int, jwt: JWT):
        return self.shard.pool_all_request(guild_id, PoolType.AUTO_RESPONSE, routing_guild=guild_id)

    @reqparams(trigger=str, response=str)
    @authenticated()
    def post(self, guild_id: int, trigger: str, response: str, jwt: JWT):
        return self.shard.set_response(jwt.id, guild_id, trigger, response, routing_guild=guild_id)

    @reqparams(trigger=str)
    @authenticated()
    def delete(self, guild_id: int, trigger: str, jwt: JWT):
        return self.shard.delete_response(jwt.id, guild_id, trigger, routing_guild=guild_id)

    @reqparams(trigger=str, response=str)
    @authenticated()
    def patch(self, guild_id: int, trigger: str, response: str, jwt: JWT):
        _, sc = self.shard.delete_response(jwt.id, guild_id, trigger, routing_guild=guild_id)

        return self.shard.set_response(jwt.id, guild_id, trigger, response, routing_guild=guild_id)


class Settings(CustomResource):
    @authenticated(member=True)
    def get(self, guild_id: int, setting: str = None, jwt: JWT = None):
        if setting is None:
            with open('settings.json') as f:
                return json.loads(f.read()), 200
        # discord_id = authenticate(self.session, request.headers).discord_id
        return self.shard.settings_access(guild_id, setting, None, routing_guild=guild_id)

    def post(self, guild_id: int, setting: str):
        return StatusCodes.BAD_REQUEST_400


class Coggers(CustomResource):
    '''provide an endpoint to reload cogs in the bot'''
    def get(self, extension: str = None):
        return self.shard.get_extensions()

    def post(self, extension: str):
        return self.shard.reload_extension(extension, routing_guild="all")


class Stats(CustomResource):
    @authenticated(member=True)
    def get(self, guild_id: int, jwt: JWT):
        '''Request message count statistics from shard and return'''
        data, sc = self.shard.bin_messages(guild_id, jwt.id, routing_guild=guild_id)
        camelcase_keys(data)
        return data, sc


class Music(CustomResource):
    @authenticated(member=True)
    def get(self, guild_id: int, jwt: JWT):
        resp, sc = self.shard.get_playlist(guild_id, routing_guild=guild_id)
        return camelcase_keys(resp), sc

    @reqparams(song=str)
    @authenticated(member=True)
    def post(self, guild_id: int, song: str, jwt: JWT):
        return self.shard.queue_song(guild_id, int(jwt.id), song, routing_guild=guild_id)


class Emoji(CustomResource):

    def get(self, emoji_id: int):
        result = self.session.execute('''SELECT img FROM tb_emojis WHERE id = :id''', {'id': emoji_id}).fetchone()
        if result is None:
            return "emoji not found", StatusCodes.NOT_FOUND_404
        return Response(FileWrapper(BytesIO(result['img'])), mimetype="text/plain", direct_passthrough=True)

    @authenticated(member=True)
    def post(self, guild_id: int, emoji_id: int, jwt: JWT):
        return self.shard.load_emoji(guild_id, emoji_id, jwt.id, routing_guild=guild_id)

    @authenticated(member=True)
    def patch(self, guild_id: int, emoji_id: int, jwt: JWT):
        return self.shard.cache_emoji(guild_id, emoji_id, jwt.id, routing_guild=guild_id)

    @authenticated(member=True)
    def delete(self, guild_id: int, emoji_id: int, jwt: JWT):
        return self.shard.delete_emoji(guild_id, emoji_id, jwt.id, routing_guild=guild_id)


class ListGuilds(CustomResource):
    @authenticated()
    def get(self, jwt: JWT):
        '''Forward guild list request to discord and return response'''
        resp, status_code = list_guilds_request(jwt)
        if status_code == StatusCodes.OK_200:
            resp, _ = self.shard.tag_autbot_guilds(resp, jwt.id)
        return resp, status_code


class Twitch(CustomResource):
    def get(self):
        challenge = request.args.get("hub.challenge")
        return make_response(challenge, StatusCodes.OK_200)

    @verify_twitch_hub
    def post(self):
        if request.json is None or 'data' not in request.json:
            return StatusCodes.BAD_REQUEST_400

        # twitch can send the same event more than once.
        id = request.headers['Twitch-Notification-Id']
        seen_before = self.redis.getset(id, '1') is not None
        self.redis.expire(id, 60)
        if seen_before:
            return StatusCodes.NO_CONTENT_204

        # send update to all the relevant shards
        for stream in request.json['data']:
            user_id = stream["user_id"]
            result = self.session.execute(
                '''SELECT guild_id FROM tb_twitch_subs WHERE stream_user_id = :stream_user_id''',
                {'stream_user_id': int(user_id)}).fetchall()
            ids = {which_shard(row['guild_id']) for row in result}

            for shard_id in ids:
                try:
                    self.shard.client.call('twitch_update', stream, routing_key=f'shard_rpc_{shard_id}')
                except Exception:
                    logger.exception(f"Error forwarding twitch update to shard {shard_id}")


@app.route('/status')
def status():
    return "all systems operational", StatusCodes.NO_CONTENT_204


def app_factory():
    api = Api(app)
    api.add_resource(Identify, "/session/identify")
    api.add_resource(Login, "/session/login")
    api.add_resource(RefreshToken, "/session/refresh")
    api.add_resource(End, "/session/end")
    api.add_resource(TokenExchange, "/session/token-exchange")

    api.add_resource(Checkout, "/checkout")

    api.add_resource(AllGuilds, "/admin/guilds")
    api.add_resource(User, "/user/<string:name>")
    api.add_resource(Settings, "/settings/<int:guild_id>/<string:setting>", "/settings/<int:guild_id>")
    api.add_resource(ListGuilds, "/guilds")
    api.add_resource(Stats, "/stats/<int:guild_id>")
    api.add_resource(Music, "/music/<int:guild_id>")
    api.add_resource(Emoji, "/emojis/<int:emoji_id>", "/emojis/<int:guild_id>/<int:emoji_id>")
    api.add_resource(AutoResponses, "/responses/<int:guild_id>")
    api.add_resource(Logs, "/logs/<int:guild_id>")
    api.add_resource(RedirectCallback, "/redirect")
    api.add_resource(GuildCounter, "/guild-count")
    api.add_resource(Invite, "/invite/<int:guild_id>")
    api.add_resource(Twitch, "/twitch")
    if not is_prod:
        api.add_resource(Coggers, "/coggers/<string:extension>", "/coggers")
    return app
