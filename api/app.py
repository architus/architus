from api import app_factory

application = app_factory(q)
if __name__ == '__main__':
    application.run()
