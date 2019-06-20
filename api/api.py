from flask import Flask, redirect
from flask_restful import Api, Resource, reqparse
import time
import json
import zmq
import os

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
class user(Resource):

    def __init__(self, q=None):
        self.q = q
        ctx = zmq.Context()
        self.topic = str(os.getpid())
        self.sub = ctx.socket(zmq.SUB)
        self.sub.connect("tcp://127.0.0.1:7208")
        self.sub.setsockopt(zmq.SUBSCRIBE, self.topic.encode())

    def get(self, name):
        print("sending from api.py")
        self.q.put(json.dumps({'method' : "fetch_user", 'arg' : name, 'topic' : self.topic}))
        name = self.sub.recv().decode().replace(self.topic + ' ', '')
        return f"{name}", 201

    def post(self, name):
        return "not implemented", 418

    def put(self, name):
        return "not implemented", 418

    def delete(self, name):
        return "not implemented", 418



if __name__ == '__main__':
    app.run(debug=False, port=5050)
