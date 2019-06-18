from api.api import application
from flask_restful import Api
from api.api import autbot
from multiprocessing import Process, Pipe, Queue
import bot2
from bot2 import coolbot


print("wsgi loaded")

api_conn, bot_conn = Pipe()
api = Api(application)
api.add_resource(autbot, "/autbot/<string:name>", resource_class_kwargs={'conn': api_conn})

coolbot.conn = bot_conn
p = Process(target=coolbot.run, args=('NDQ4OTQwOTgwMjE4MTAxNzk1.Dedc_g.gVJWPgJVlrhh6j5NL9nbZpMGUhI',))
p.start()
if __name__ == '__main__':
    application.run()
    p.join()
