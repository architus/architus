import grpc
from grpc._channel import _InactiveRpcError
from grpc._channel import _MultiThreadedRendezvous
from lib.ipc import manager_pb2_grpc

from lib.config import logger
import asyncio
import time
from concurrent.futures import ThreadPoolExecutor

grpc_options = (
    ('grpc.keepalive_time_ms', 10000),
    ('grpc.keeepalive_timeout_ms', 5000),
    ('grpc.keepalive_permit_without_calls', True),
    ('grpc.http2.max_pings_without_data', 0),
    ('grpc.http2.min_time_between_pings_ms', 10000),
    ('grpc.http2.min_ping_interval_without_data_ms', 5000)
)


class SyncRPCClient():
    def __init__(self, stub):
        self.stub = stub

    def rpc(self, f, a):
        while True:
            try:
                return f(a)
            except _InactiveRpcError:
                continue

    def register(self, v):
        return self.rpc(self.stub.register, v)

    def guild_count(self, v):
        return self.rpc(self.stub.guild_count, v)

    def checkin(self, i):
        return self.rpc(self.stub.checkin, i)

    def publish_file(self, f):
        return self.rpc(self.stub.publish_file, f)

    def all_guilds(self, v):
        return self.rpc(self.stub.all_guilds, v)

    def guild_update(self, g):
        return self.rpc(self.stub.guild_update, g)


def get_blocking_client():
    while True:
        try:
            channel = grpc.insecure_channel('manager:50051', options=grpc_options)
            stub = manager_pb2_grpc.ManagerStub(channel)
            return SyncRPCClient(stub)
        except Exception as e:
            logger.debug(f"Waiting to connect to gRPC {e}")
            time.sleep(3)

# TODO: gRPC will hopefully be releasing actual support for python async soon.
#       Will need to update to actually take advantage of that when it comes out.


class AsyncRPCClient():
    def __init__(self, stub):
        self.stub = stub
        self.loop = asyncio.get_event_loop()
        self.pool = ThreadPoolExecutor(max_workers=8)

    async def rpc(self, f, a):
        while True:
            try:
                return await self.loop.run_in_executor(self.pool, f, a)
            except _InactiveRpcError:
                continue
            except _MultiThreadedRendezvous:
                continue

    async def register(self, v):
        return await self.rpc(self.stub.register, v)

    async def guild_count(self, v):
        return await self.rpc(self.stub.guild_count, v)

    async def checkin(self, i):
        return await self.rpc(self.stub.checkin, i)

    async def publish_file(self, f):
        return await self.rpc(self.stub.publish_file, f)

    async def all_guilds(self, v):
        return await self.rpc(self.stub.all_guilds, v)

    async def guild_update(self, g):
        return await self.rpc(self.stub.guild_update, g)


def get_async_client():
    print("called get async client")
    stub = None
    while True:
        try:
            channel = grpc.insecure_channel('manager:50051', options=grpc_options)
            stub = manager_pb2_grpc.ManagerStub(channel)
            logger.debug("Connected to gRPC")
            break
        except Exception:
            logger.debug("Waiting to connect to gRPC")
            time.sleep(3)

    return AsyncRPCClient(stub)
