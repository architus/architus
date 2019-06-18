from api.api import application
from flask_restful import Api
from api.api import autbot
from multiprocessing import Process, Pipe, Queue
import bot2
from bot2 import coolbot

#export PYTHON=python3.6; uwsgi --ini config_uwsgi.ini --http :5061 --wsgi-file wsgi.py
print("wsgi loaded")

api_conn, bot_conn = Pipe()
api = Api(application)
api.add_resource(autbot, "/autbot/<string:name>", resource_class_kwargs={'conn': api_conn})

coolbot.conn = bot_conn
p = Process(target=coolbot.run, args=('',))
p.start()
if __name__ == '__main__':
    application.run()
    #p.join()
