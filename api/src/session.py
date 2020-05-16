from datetime import datetime, timedelta
from urllib.parse import quote_plus
from secrets import randbits

from flask_restful import Resource
from flask import redirect, request

from lib.config import REDIRECT_URI, client_id, domain_name as DOMAIN
from lib.status_codes import StatusCodes
from lib.auth import JWT, flask_authenticated as authenticated
from lib.config import logger

from src.util import CustomResource, reqparams, time_to_refresh
from src.discord_requests import identify_request, token_exchange_request, refresh_token_request

SAFE_REDIRECT_URI = quote_plus(REDIRECT_URI)


def make_token_cookie_header(token: str, max_age: int) -> dict:
    '''creates a set-cookie header for storing the auth token'''
    return {'Set-Cookie': f'token={token}; Max-Age={max_age}; Path=/; Domain=api.{DOMAIN}; Secure; HttpOnly'}


def generate_refresh_response(jwt: JWT) -> tuple:
    '''makes a refresh request and compiles the json, status code, and set-cookie header'''
    data, sc = refresh_token_request(jwt.refresh_token)
    if sc == StatusCodes.OK_200:
        now = datetime.now()
        jwt.access_token = data['access_token']
        jwt.refresh_token = data['refresh_token']
        jwt.expires_in = data['expires_in']
        jwt.issued_at = now.isoformat()

        return {
            'access': {
                'issuedAt': now.isoformat(),
                'expiresIn': jwt.expires_in,
                'refresh_in': jwt.expires_in // 2
            }
        }, sc, make_token_cookie_header(jwt.get_token(), jwt.expires_in * 2)
    return data, sc


class Login(CustomResource):
    def get(self):
        response = redirect(f'https://discordapp.com/api/oauth2/authorize?client_id={client_id}&redirect_uri='
                            f'{SAFE_REDIRECT_URI}&response_type=code&scope=identify%20guilds')
        # TODO check if requested return url is owned by us
        # if not any(re.match(pattern, url) for pattern in (
        #         r'https:\/\/[-A-Za-z0-9]{24}--architus\.netlify\.com\/app',
        #         r'https:\/\/deploy-preview-[0-9]+--architus\.netlify\.com\/app',
        #         r'https:\/\/develop\.archit\.us\/app',
        #         r'https:\/\/archit\.us\/app',
        #         r'http:\/\/localhost:3000\/app')):
        #     url = CALLBACK_URL

        # TODO default destination if return is not included
        response.set_cookie(
            'next',
            request.args.get('return') or f'https://{DOMAIN}/app',
            domain=f'api.{DOMAIN}',
            secure=True, httponly=True
        )
        return response


class End(CustomResource):
    @authenticated
    def post(self, jwt: JWT):
        self.shard.client.call(
            'demote_connection',
            jwt.get_token(),
            routing_key='gateway_rpc'
        )
        return {'message': 'ok I definitetly did something :)'}, StatusCodes.OK_200, make_token_cookie_header(None, -1)


class RefreshToken(CustomResource):
    @authenticated
    def post(self, jwt: JWT):
        if time_to_refresh(jwt):
            return generate_refresh_response(jwt)
        return {'message': 'It\'s not time to refresh your token'}, StatusCodes.TOO_MANY_REQUESTS_429


class TokenExchange(CustomResource):
    @reqparams(code=str)
    def post(self, code: str):
        ex_data, status_code = token_exchange_request(code)

        if status_code == StatusCodes.OK_200:
            discord_token = ex_data['access_token']
            id_data, status_code = identify_request(discord_token)
            if status_code == StatusCodes.OK_200:
                now = datetime.now()
                expires_in = ex_data['expires_in']
                refresh_in = timedelta(seconds=expires_in) / 2
                jwt = JWT({
                    'access_token': discord_token,
                    'refresh_token': ex_data['refresh_token'],
                    'expires_in': expires_in,
                    'issued_at': now.isoformat(),
                    'id': int(id_data['id']),
                    'permissions': 274,
                })
                nonce = randbits(32)
                data = {
                    # 'token': jwt.get_token()
                    'user': id_data,
                    'access': {
                        'issuedAt': now.isoformat(),
                        'expiresIn': expires_in,
                        'refreshIn': int(refresh_in.total_seconds()),
                    },
                    'gatewayNonce': nonce,
                }

                self.shard.client.call(
                    'register_nonce',
                    nonce,
                    jwt.get_token(),
                    routing_key='gateway_rpc'
                )
                return data, StatusCodes.OK_200, make_token_cookie_header(jwt.get_token(), expires_in * 2)

        return ex_data, status_code


class Identify(Resource):
    @authenticated
    def get(self, jwt: JWT):
        '''Forward identify request to discord and return response'''
        id_data, sc = identify_request(jwt.access_token)
        if sc == StatusCodes.OK_200:
            if time_to_refresh(jwt):
                data, *rest = generate_refresh_response(jwt)
                data['user'] = id_data
                logger.debug((data, *rest))
                return (data, *rest)
            else:
                a = {
                    'user': id_data,
                    'access': {
                        'issuedAt': jwt.issued_at,
                        'expiresIn': jwt.expires_in,
                        'refreshIn': jwt.expires_in // 2,
                    }
                }, StatusCodes.OK_200
                logger.debug(a)
                return a
        logger.debug((id_data, sc))
        return id_data, sc
