from api.api import app_factory
from multiprocessing import Process, Queue
from src.config import secret_token
from bot import coolbot

# export PYTHON=python3.6; uwsgi --ini config_uwsgi.ini --http :5061 --wsgi-file wsgi.py
print("WSGI LOADED")

application = app_factory(q)

p = Process(name="autbot", target=coolbot.run, args=(secret_token,), kwargs={'q': q})
p.start()

if __name__ == '__main__':
    application.run()
