import grpc
from grpc._channel import _InactiveRpcError
from grpc._channel import _MultiThreadedRendezvous
from lib.ipc import manager_pb2_grpc

from lib.config import logger
import asyncio
import time

class SyncRPCClient():
    def __init__(self, stub):
        self.stub = stub

    def rpc(self, f, a):
        while True:
            try:
                return f(a)
            except _InactiveRpcError as e:
                continue

    def register(self, v):
        return self.rpc(self.stub.register, v)

    async def guild_count(self, v):
        return self.rpc(self.stub.guild_count, v)

    async def checkin(self, i):
        return self.rpc(self.stub.checkin, i)

    async def publish_file(self, f):
        return self.rpc(self.stub.publish_file, f)

    async def all_guilds(self, v):
        return self.rpc(self.stub.all_guilds, v)

    async def guild_update(self, g):
        return self.rpc(self.stub.guild_update, g)

def get_blocking_client():
    while True:
        try:
            channel = grpc.insecure_channel('manager:50051')
            stub = manager_pb2_grpc.ManagerStub(channel)
            return SyncRPCClient(stub)
        except Exception as e:
            logger.debug(f"Waiting to connect to gRPC {e}")
            time.sleep(3)

class AsyncRPCClient():
    def __init__(self, stub):
        self.stub = stub

    async def rpc(self, f, a):
        while True:
            try:
                return f(a).result()
            except _InactiveRpcError as e:
                continue
            except _MultiThreadedRendezvous as m:
                continue

    async def register(self, v):
        return await self.rpc(self.stub.register.future, v)

    async def guild_count(self, v):
        return await self.rpc(self.stub.guild_count.future, v)

    async def checkin(self, i):
        return await self.rpc(self.stub.checkin.future, i)

    async def publish_file(self, f):
        return await self.rpc(self.stub.publish_file.future, f)

    async def all_guilds(self, v):
        return await self.rpc(self.stub.all_guilds.future, v)

    async def guild_update(self, g):
        return await self.rpc(self.stub.guild_update.future, g)


def get_async_client():
    connected = False
    while not connected:
        try:
            channel = grpc.insecure_channel('manager:50051')
            stub = manager_pb2_grpc.ManagerStub(channel)
            logger.debug("Connected to gRPC")
            connected = True
            return AsyncRPCClient(stub)
        except:
            logger.debug("Waiting to connect to gRPC")
            time.sleep(3)
