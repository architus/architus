import asyncio
import zmq
import zmq.asyncio

print("starting apitest")

@asyncio.coroutine
def api_listener(socket):
    msg = yield from socket.recv_string()
    yield from asyncio.sleep(2)
    yield from socket.send_string(f"message was {msg}")
    socket.close()


@asyncio.coroutine
def connection_listener():
    context = zmq.asyncio.Context()
    socket = context.socket(zmq.REP)
    socket.bind('tcp://127.0.0.1:7100')
    while True:
        msg = yield from socket.recv_string()
        print(msg)
        new_socket = context.socket(zmq.REP)
        port = new_socket.bind_to_random_port('tcp://127.0.0.1')
        print(f"opened new connection on {port}")
        asyncio.ensure_future(api_listener(new_socket))
        yield from socket.send(int.to_bytes(port, length=5, byteorder='little'))

asyncio.get_event_loop().run_until_complete(connection_listener())
