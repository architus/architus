from api.api import application
from flask_restful import Api
from api.api import user
from multiprocessing import Process, Pipe, Queue
#from asyncio import Queue
import bot2
from bot2 import coolbot
from src.config import secret_token
import zmq

#export PYTHON=python3.6; uwsgi --ini config_uwsgi.ini --http :5061 --wsgi-file wsgi.py
print("wsgi loaded")

ctx = zmq.Context()
pub = ctx.socket(zmq.PUB)
#pub.bind("tcp://127.0.0.1:7100")

q = Queue()
api = Api(application)
api.add_resource(user, "/user/<string:name>", resource_class_kwargs={'conn_q' : (q, ctx)})

coolbot.q = q
p = Process(target=coolbot.run, args=(secret_token,))
p.start()
if __name__ == '__main__':
    application.run()
    #p.join()
