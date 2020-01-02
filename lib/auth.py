import jwt as pyjwt
from flask import request
from lib.status_codes import StatusCodes


def flask_authenticated(func):
    """decorator for rest endpoint functions
    returns 401 if user is not logged in
    and prepends a JWT object to the kwargs for id data
    """
    def authed_func(self, *args, **kwargs):
        try:
            jwt = JWT(token=request.cookies.get('token'))
            # TODO check token expiration
        except pyjwt.exceptions.InvalidTokenError:
            return (StatusCodes.UNAUTHORIZED_401, "Not Authorized")
        return func(self, *args, jwt=jwt, **kwargs)
    return authed_func


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
