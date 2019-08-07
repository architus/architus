import zmq


def main():
    try:
        context = zmq.Context(1)

        # Socket facing api
        frontend = context.socket(zmq.XREP)
        frontend.bind("tcp://0.0.0.0:7300")
        # Socket facing bot
        backend = context.socket(zmq.XREQ)
        backend.bind("tcp://0.0.0.0:6300")

        zmq.device(zmq.QUEUE, frontend, backend)
    except Exception as e:
        print(f"{e} - bringing down zmq device")
    finally:
        frontend.close()
        backend.close()
        context.term()


if __name__ == "__main__":
    main()
