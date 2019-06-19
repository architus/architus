from flask import Flask
from quart import Quart
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


class user(Resource):

    def __init__(self, conn_q=None):
        self.q = conn_q[0]
        ctx = zmq.Context()
        self.topic = os.getpid()
        self.sub = ctx.socket(zmq.SUB)
        self.sub.connect("tcp://127.0.0.1:7208")
        self.sub.setsockopt(zmq.SUBSCRIBE, str(self.topic).encode())

    def get(self, name):
        print("sending from api.py")
        self.q.put(json.dumps({'method' : "fetch_user", 'arg' : name, 'topic' : self.topic}))
        name = self.sub.recv()
        return f"{name}", 201

    def post(self, name):
        return "not implemented", 418

    def put(self, name):
        return "not implemented", 418

    def delete(self, name):
        return "not implemented", 418



if __name__ == '__main__':
    app.run(debug=False, port=5050)
