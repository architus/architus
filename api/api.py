from flask import Flask
from flask_restful import Api, Resource, reqparse
import zmq

app = Flask(__name__)
api = Api(app)

context = zmq.Context()
socket = context.socket(zmq.REQ)
socket.connect('tcp://127.0.0.1:7100')


class autbot(Resource):

    def get(self, name):
        print("got " + name)
        socket.send_string(name)
        msg = socket.recv().decode("ascii")
        print("got " + msg)
        return f"{msg}", 201

    def post(self, name):
        return "not implemented", 418

    def put(self, name):
        return "not implemented", 418

    def delete(self, name):
        return "not implemented", 418


api.add_resource(autbot, "/autbot/<string:name>")

app.run(debug=False, port=5050)
