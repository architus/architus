from api import app_factory

application = app_factory()
if __name__ == '__main__':
    application.run(host='0.0.0.0')
