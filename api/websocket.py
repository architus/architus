import websockets
import asyncio
import zmq
import zmq.asyncio
import functools

TOPIC = 101010

def enqueue(call, q):
    call['topic'] = TOPIC
    q.put(json.dumps(call))

def recv(sub):
    return json.loads(sub.recv().decode().replace(TOPIC + ' ', ''))

async def handle_socket(websocket, path, q=None, sub=None):
    try:
        data = json.loads(await websocket.recv())
        self.enqueue({
            'method': "interpret",
            'args': [data['guild_id'], data['message']]
        }, q)
        resp = recv(sub)
    except Exception as e:
        traceback.print_exc()
        print(f"caught {e} while handling websocket request")
        resp = {'message': str(e)}
    await websocket.send(json.dumps(resp))

def not_run(q=None):
    sub = None
    #ctx = zmq.asyncio.Context()
    #sub = ctx.socket(zmq.SUB)
    #sub.connect("tcp://127.0.0.1:7200")
    bound_handler = functools.partial(handle_socket, q=q, sub=sub)
    start_server = websockets.serve(bound_handler, 'localhost', 8300)
    asyncio.get_event_loop().run_until_complete(start_server)
    asyncio.get_event_loop().run_forever()

if __name__ == '__main__':
    not_run()
