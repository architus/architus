from flask import Flask, redirect, request
from flask_restful import Api, Resource, reqparse
from flask_cors import CORS
import requests
import time
import json
import zmq
import os
import secrets
from datetime import datetime, timedelta
from sqlalchemy.exc import IntegrityError

from src.config import client_id, client_secret, get_session
from src.models import AppSession

API_ENDPOINT = 'https://discordapp.com/api/v6'
REDIRECT_URI = 'https://aut-bot.com/home'

application = Flask(__name__)
cors = CORS(application)

@application.route('/login')
def login():
    return redirect('https://discordapp.com/api/oauth2/authorize?client_id=448546825532866560&redirect_uri=https%3A%2F%2Faut-bot.com%2Fhome&response_type=code&scope=identify')

class CustomResource(Resource):
    def __init__(self, q=None):
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


class user(CustomResource):

    def get(self, name):
        self.enqueue({'method': "fetch_user_dict", 'args': [name]})
        name = self.recv()
        return name, 200

    def post(self, name):
        return "not implemented", 418

class Interpret(CustomResource):

    def post(self):
        parser = reqparse.RequestParser()
        parser.add_argument('message')
        args = parser.parse_args()
        if not 'message' in args:
            return 400
        self.enqueue({'method': "interpret", 'args': [args['message']]})
        resp = self.recv()
        print(resp)
        if 'response' in resp:
            return resp, 200
        return resp, 204

class identify(Resource):

    def get(self):
        session = get_session()
        headers = request.headers
        try:
            autbot_token = headers['Authorization']
        except KeyError:
            return "missing token", 401
        rows = session.query(AppSession).filter_by(autbot_access_token=autbot_token).all()
        for row in rows:
            if datetime.now() < row.autbot_expiration:
                data, code = discord_identify_request(row.discord_access_token)
                return data, code

        return "token invalid or expired", 401


def commit_tokens(autbot_token, discord_token, refresh_token, expires_in):
    session = get_session()
    time = datetime.now() + timedelta(seconds=int(expires_in) - 60)
    new_appsession = AppSession(autbot_token, discord_token, refresh_token, time, time, datetime.now())
    session.add(new_appsession)
    session.commit()


def discord_identify_request(token):
    headers = {
        'Content-Type': 'application/x-www-form-urlencoded',
        'Authorization': f"Bearer {token}"
    }
    r = requests.get('%s/users/@me' % API_ENDPOINT, headers=headers)
    return r.json(), r.status_code


@application.route('/token_exchange', methods=['POST'])
def token_exchange():
    print(os.getpid())
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
    if r.status_code == 200:
        print(resp_data)

        discord_token = resp_data['access_token']
        autbot_token = secrets.token_urlsafe()
        expires_in = resp_data['expires_in']
        commit_tokens(autbot_token, discord_token, resp_data['refresh_token'], expires_in)

        print("trying to get me")
        resp_data, status_code = discord_identify_request(discord_token)
        if status_code == 200:
            print(resp_data)
            return json.dumps({'access_token': autbot_token, 'expires_in': expires_in, 'username': resp_data['username'], 'discriminator': resp_data['discriminator'], 'avatar': resp_data['avatar'], 'id': resp_data['id']}), 200
    return "invalid code", 401
