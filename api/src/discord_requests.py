import requests

from lib.config import client_id, client_secret, REDIRECT_URI, API_ENDPOINT


def token_exchange_request(code):
    data = {
        'client_id': client_id,
        'client_secret': client_secret,
        'grant_type': 'authorization_code',
        'code': code,
        'redirect_uri': REDIRECT_URI,
        'scope': 'identify',
    }

    headers = {'Content-Type': 'application/x-www-form-urlencoded'}
    r = requests.post('%s/oauth2/token' % API_ENDPOINT, data=data, headers=headers)

    return r.json(), r.status_code


def identify_request(token):
    headers = {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Authorization': f"Bearer {token}"
    }
    r = requests.get('%s/users/@me' % API_ENDPOINT, headers=headers)

    return r.json(), r.status_code


def list_guilds_request(jwt):
    headers = {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Authorization': f"Bearer {jwt.access_token}"
    }
    r = requests.get('%s/users/@me/guilds' % API_ENDPOINT, headers=headers)

    return r.json(), r.status_code
