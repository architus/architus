from flask import Flask
from quart import Quart
from flask_restful import Api, Resource, reqparse
import time
import zmq

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


class autbot(Resource):

    def __init__(self, conn):
        self.conn = conn

    def get(self, name):
        print("new get")
        #new_socket = request_socket(context)
        #new_socket.send_string(name)
        #msg = new_socket.recv_string()
        #new_socket.close()
        self.conn.send(name)
        time.sleep(1)
        return f"{name}", 201

    def post(self, name):
        return "not implemented", 418

    def put(self, name):
        return "not implemented", 418

    def delete(self, name):
        return "not implemented", 418



if __name__ == '__main__':
    app.run(debug=False, port=5050)
