import jwt as pyjwt
from flask import request
from datetime import datetime, timedelta
from lib.status_codes import StatusCodes
from lib.config import jwt_secret, twitch_hub_secret, logger, is_prod
from functools import wraps
import hmac
import hashlib


def get_valid_jwt(cookies):
    try:
        if is_prod:
            token = cookies['token']
        else:
            token = cookies['dev-token']
    except KeyError:
        return None
    try:
        return JWT(token=token)
    except pyjwt.exceptions.InvalidSignatureError:
        bad_jwt = JWT(token=token, verify_signature=False)
        logger.info(f'{bad_jwt.id} attempted to access {request.path} with an invalid signature')
    except pyjwt.exceptions.InvalidTokenError:
        logger.info('error decoding jwt attempting to access {request.path}')


def flask_authenticated(member=False, admin=False):
    """decorator for rest endpoint functions
    returns 401 if user is not logged in
    and prepends a JWT object to the kwargs for id data
    if member is True, checks if the logged in user is a member of the guild
    passed in the via kwargs['guild_id']
    """
    def decorator(func):
        @wraps(func)
        def wrapper(self, *args, **kwargs):
            jwt = get_valid_jwt(request.cookies)
            if jwt is None:
                return ({'message': "Not Authorized"}, StatusCodes.UNAUTHORIZED_401)
            if expired(jwt):
                logger.info(f'{jwt.id} attempted to access {request.path} with an expired token')
                return ({'message': "Expired"}, StatusCodes.UNAUTHORIZED_401)
            if member:
                data, sc = self.shard.is_member(jwt.id, kwargs['guild_id'], routing_guild=kwargs['guild_id'])
                if sc != 200 or not data['member']:
                    logger.info(f'{jwt.id} attempted to access {request.path} but was not a member')
                    return ({'message': "Not Authorized"}, StatusCodes.UNAUTHORIZED_401)
            if admin:
                if jwt.id not in (214037134477230080,):
                    return 'admins only', StatusCodes.FORBIDDEN_403
            return func(self, *args, **kwargs, jwt=jwt)
        return wrapper
    return decorator


def gateway_authenticated(shard, member=False):
    def decorator(func):
        @wraps(func)
        async def wrapper(self, sid, data, *args, **kwargs):
            async with self.session(sid) as session:
                try:
                    jwt = session['jwt']
                except KeyError as e:
                    await self.emit('error', {
                        'message': 'not authenticated',
                        'human': 'There was an error authenticating your request.',
                        'details': 'session did not contain jwt',
                        'context': [str(e)],
                        'code': 401,
                    }, room=sid)
                    return

                if member:
                    resp, sc = shard.is_member(jwt.id, data['guild_id'], routing_guild=data['guild_id'])
                    if sc != 200 or not resp['member']:
                        await self.emit('error', {
                            'message': 'not a member',
                            'human': 'Unable to verify membership of the server.',
                            'details': 'shard reported not a member',
                            'context': [resp],
                            'code': 401,
                        }, room=sid)
                        return
                return await func(self, sid, data, *args, **kwargs, jwt=jwt)
        return wrapper
    return decorator


def verify_twitch_hub(func):
    @wraps(func)
    def wrapper(self, *args, **kwargs):
        try:
            signature = request.headers['x-hub-signature'].split('=')
            digest = hmac.new(twitch_hub_secret.encode(), msg=request.data, digestmod=hashlib.sha256).hexdigest()
            if not hmac.compare_digest(digest, signature[1]):
                logger.info("Request had invalid signature")
                return ({'message': "Invalid Signature"}, StatusCodes.UNAUTHORIZED_401)
        except KeyError:
            return ({'message': "Signature Required"}, StatusCodes.UNAUTHORIZED_401)
        except Exception:
            logger.exception("error verifying twitch hub request")
            return ({'message': "Unknown error"}, StatusCodes.INTERNAL_SERVER_ERROR_500)
        return func(self, *args, **kwargs)
    return wrapper


class JWT:
    def __init__(self, data=None, token=None, verify_signature=True):
        if data is None and token is None:
            raise pyjwt.exceptions.InvalidTokenError("Token (or data) is empty")

        if data is None:
            self._data = self._decode(token, verify_signature)
        else:
            self._data = data

        self._token = token
        self._dirty = token is None

    def get_token(self):
        if self._dirty:
            try:
                self._token = self._encode(self._data).decode()
            except AttributeError:
                self._token = self._encode(self._data)
            self._dirty = False
        return self._token

    def __getattr__(self, name):
        try:
            return self._data[name]
        except KeyError:
            raise AttributeError(f"no such attribute {name}") from None

    def _decode(self, token, verify=True):
        return pyjwt.decode(token, jwt_secret, algorithms=['HS256'], options={'verify_signature': verify})

    def _encode(self, payload):
        return pyjwt.encode(payload, jwt_secret, algorithm='HS256')


def expired(jwt: JWT):
    issued_at = datetime.strptime(jwt.issued_at, "%Y-%m-%dT%H:%M:%S.%f")
    expires_in = timedelta(seconds=jwt.expires_in)
    return datetime.now() > issued_at + expires_in
