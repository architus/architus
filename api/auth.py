import jwt as pyjwt

# from lib.config import jwt_secret
from lib.status_codes import StatusCodes


def authenticated(func):
    """decorator for rest endpoint functions
    returns 401 if user is not logged in
    and prepends a JWT object to the kwargs for id data
    """
    def authed_func(self, *args, **kwargs):
        try:
            jwt = JWT(jwt=request.headers['Authorization'])
        except jwt.exceptions.InvalidTokenError:
            return (StatusCodes.UNAUTHORIZED_401, "Not Authorized")
        return func(self, *args, jwt=jwt, **kwargs)
    return authed_func


def token_exchange_request(code):
    data = {
        'client_id': client_id,
        'client_secret': client_secret,
        'grant_type': 'authorization_code',
        'code': args['code'],
        'redirect_uri': REDIRECT_URI,
        'scope': 'identify',
    }
    headers = {'Content-Type': 'application/x-www-form-urlencoded'}
    r = requests.post('%s/oauth2/token' % API_ENDPOINT, data=data, headers=headers)
    return r.json(), r.status_code

def discord_identify_request(token):
    headers = {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Authorization': f"Bearer {token}"
    }
    r = requests.get('%s/users/@me' % API_ENDPOINT, headers=headers)
    return r.json(), r.status_code


class JWT:
    def __init__(self, data=None, jwt=None):
        assert data is not None or jwt is not None

        if data is None:
            self._data = self._decode(jwt)
        else:
            self._data = data

        self._jwt = jwt
        self._dirty = jwt is None

    def get_token(self):
        if self._dirty:
            self._jwt = self._encode(self.data)
            self._dirty = False
        return self._jwt

    def __getattr__(self, name):
        try:
            return self._data[name]
        except KeyError:
            raise AttributeError(f"no such attribute {name}") from None

    def __setattr__(self, name, value):
        self._dirty = True
        self._data[name] = value

    def _decode(self, jwt):
        data = pyjwt.decode(encoded_jwt, 'secret', alorgithms='HS256')

    def _encode(self, payload):
        return pyjwt.encode(payload, 'secret', algorithm='HS256')
