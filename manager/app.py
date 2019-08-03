import asyncio
import websockets
import json
import os

NUM_SHARDS = int(os.environ['NUM_SHARDS'])
print(f"Number of shards: {NUM_SHARDS}")
current_shard_id = 0


async def handle_shard(websocket, path):
    global current_shard_id
    if current_shard_id < NUM_SHARDS:
        shard_id = current_shard_id
        current_shard_id += 1
        print(f"INFO - assigning shard {shard_id}")
    else:
        print(f"WARNING - bot asking for shard id, but we're already at the max ({NUM_SHARDS})")
        shard_id = -1
    content = {
        'num_shards': NUM_SHARDS,
        'shard_id': shard_id
    }
    await websocket.send(json.dumps(content))
    resp = await websocket.recv()

    while True:
        try:
            await asyncio.sleep(100)
            await websocket.send("hb")
            resp = await websocket.recv()
            print(f"INFO - shard {shard_id} responded to heartbeat with: {resp}")
        except websockets.exceptions.ConnectionClosed as e:
            print(e)
            print(f"WARNING - shard {shard_id} is DOWN")
            return

print("monitoring server listening on 5300")
start_server = websockets.serve(handle_shard, "0.0.0.0", 5300)
asyncio.get_event_loop().run_until_complete(start_server)
asyncio.get_event_loop().run_forever()
