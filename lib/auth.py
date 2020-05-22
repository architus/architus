import jwt as pyjwt
from flask import request
from lib.status_codes import StatusCodes

from functools import wraps


def flask_authenticated(member=False):
    """decorator for rest endpoint functions
    returns 401 if user is not logged in
    and prepends a JWT object to the kwargs for id data
    if member is True, checks if the logged in user is a member of the guild
    passed in the via kwargs['guild_id']
    """
    def decorator(func):
        @wraps(func)
        def wrapper(self, *args, **kwargs):
            try:
                jwt = JWT(token=request.cookies.get('token'))
                # TODO check token expiration
            except pyjwt.exceptions.InvalidTokenError:
                return ({'message': "Not Authorized"}, StatusCodes.UNAUTHORIZED_401)
            if member:
                data, sc = self.shard.is_member(jwt.id, kwargs['guild_id'], routing_guild=kwargs['guild_id'])
                if sc != 200 or not data['member']:
                    return ({'message': "Not Authorized"}, StatusCodes.UNAUTHORIZED_401)
            return func(self, *args, **kwargs, jwt=jwt)

        return wrapper
    return decorator


class JWT:
    def __init__(self, data=None, token=None):
        if data is None and token is None:
            raise pyjwt.exceptions.InvalidTokenError("Token (or data) is empty")

        if data is None:
            self._data = self._decode(token)
        else:
            self._data = data

        self._token = token
        self._dirty = token is None

    def get_token(self):
        if self._dirty:
            self._token = self._encode(self._data)
            self._dirty = False
        return self._token.decode()

    def __getattr__(self, name):
        try:
            return self._data[name]
        except KeyError:
            raise AttributeError(f"no such attribute {name}") from None

    def _decode(self, token):
        return pyjwt.decode(token, 'secret', alorgithms='HS256')

    def _encode(self, payload):
        return pyjwt.encode(payload, 'secret', algorithm='HS256')
