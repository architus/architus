import zmq
from multiprocessing import Process

def bot_to_api():

    try:
        context = zmq.Context(1)
        # Socket facing bots
        frontend = context.socket(zmq.SUB)
        frontend.bind("tcp://*:7300")

        frontend.setsockopt(zmq.SUBSCRIBE, b'')

        # Socket facing apis
        backend = context.socket(zmq.PUB)
        backend.bind("tcp://*:6300")
        print("Shard to API forwarder listening on 7300 and 6300")

        zmq.device(zmq.FORWARDER, frontend, backend)
    except Exception as e:
        print(f"{e} - bringing down zmq device")
    finally:
        frontend.close()
        backend.close()
        context.term()

def api_to_bot():

    try:
        context = zmq.Context(1)
        # Socket facing api
        frontend = context.socket(zmq.SUB)
        frontend.bind("tcp://*:7200")

        frontend.setsockopt(zmq.SUBSCRIBE, b'')

        # Socket facing bot
        backend = context.socket(zmq.PUB)
        backend.bind("tcp://*:6200")
        print("API to Shard forwarder listening on 7200 and 6200")

        zmq.device(zmq.FORWARDER, frontend, backend)
    except Exception as e:
        print(f"{e} - bringing down zmq device")
    finally:
        frontend.close()
        backend.close()
        context.term()

if __name__ == "__main__":
    p = Process(target=api_to_bot, daemon=True)
    p.start()
    bot_to_api()
