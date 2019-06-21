from flask import Flask, redirect
from flask_restful import Api, Resource, reqparse
import requests
import time
import json
import zmq
import os

from src.config import secret_token

application = Flask(__name__)

#context = zmq.Context()
#master_socket = context.socket(zmq.REQ)
#print("conneting...")
#master_socket.connect('tcp://127.0.0.1:7100')

def request_socket(ctx):
    print("requesting port")
    master_socket.send_string("socket pls")
    port = master_socket.recv()
    print("got port " + str(port))
    port = int.from_bytes(port, byteorder='little')
    socket = ctx.socket(zmq.REQ)
    socket.connect(f'tcp://127.0.0.1:{port}')
    return socket


@application.route('/login')
def login():
    return redirect('https://discordapp.com/api/oauth2/authorize?client_id=448546825532866560&redirect_uri=https%3A%2F%2Faut-bot.com%2Fhome&response_type=code&scope=identify')

class CustomResource(Resource):
    def __init__(self, q=None):
        self.q = q
        ctx = zmq.Context()
        self.topic = str(os.getpid())
        self.sub = ctx.socket(zmq.SUB)
        self.sub.connect("tcp://127.0.0.1:7208")
        self.sub.setsockopt(zmq.SUBSCRIBE, self.topic.encode())

    def enqueue(self, call):
        call['topic'] = self.topic
        self.q.put(json.dumps(call))

    def recv(self):
        return json.loads(self.sub.recv().decode().replace(self.topic + ' ', ''))


class user(CustomResource):

    def get(self, name):
        print("sending from api.py")
        self.enqueue({'method': "fetch_user_dict", 'arg': name})
        name = self.recv()
        return name, 200

    def post(self, name):
        return "not implemented", 418


@application.route('/token_exchange', methods=['POST'])
def post():
    parser = reqparse.RequestParser()
    parser.add_argument('code')
    args = parser.parse_args()
    API_ENDPOINT = 'https://discordapp.com/api/v6'
    CLIENT_ID = '448546825532866560'
    REDIRECT_URI = 'https://aut-bot.com/home'
    data = {
        'client_id': CLIENT_ID,
        'client_secret': '',
        'grant_type': 'authorization_code',
        'code': args['code'],
        'redirect_uri': REDIRECT_URI,
        'scope': 'identify'
    }
    headers = {
        'Content-Type': 'application/x-www-form-urlencoded'
    }
    r = requests.post('%s/oauth2/token' % API_ENDPOINT, data=data, headers=headers)
    return json.dumps(r.json())


if __name__ == '__main__':
    app.run(debug=False, port=5050)
