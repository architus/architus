import grpc
from lib.ipc import manager_pb2_grpc
from lib.ipc import feature_gate_pb2_grpc

from lib.config import logger
import asyncio
import time
from concurrent.futures import ThreadPoolExecutor
from functools import partial

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

    def __getattr__(self, name):
        return getattr(self.stub, name)


def get_manager_blocking_client(server):
    while True:
        try:
            channel = grpc.insecure_channel(server, options=grpc_options)
            stub = manager_pb2_grpc.ManagerStub(channel)
            return SyncRPCClient(stub)
        except Exception as e:
            logger.debug(f"Waiting to connect to manager: {e}")
            time.sleep(3)

def get_feature_blocking_client(server):
    while True:
        try:
            channel = grpc.insecure_channel(server)
            stub = feature_gate_pb2_grpc.FeatureGateStub(channel)
            return SyncRPCClient(stub)
        except Exception as e:
            logger.debug(f"Wating to connect to feature server: {e}")
            time.sleep(3)

# TODO: gRPC will hopefully be releasing actual support for python async soon.
#       Will need to update to actually take advantage of that when it comes out.
class AsyncRPCClient():
    def __init__(self, stub):
        self.stub = stub
        self.loop = asyncio.get_event_loop()
        self.pool = ThreadPoolExecutor(max_workers=8)

    async def rpc(self, f, a):
        return await self.loop.run_in_executor(self.pool, f, a)

    def __getattr__(self, name):
        return partial(self.rpc, getattr(self.stub, name))


def get_manager_async_client(server):
    stub = None
    while True:
        try:
            channel = grpc.insecure_channel(server, options=grpc_options)
            stub = manager_pb2_grpc.ManagerStub(channel)
            logger.debug("Connected to manager")
            break
        except Exception:
            logger.debug("Waiting to connect to manager")
            time.sleep(3)

    return AsyncRPCClient(stub)

def get_feature_async_client(server):
    stub = None
    while True:
        try:
            channel = grpc.insecure_channel(server)
            stub = feature_gate_pb2_grpc.FeatureGateStub(channel)
            logger.debug("Connected to feature server")
            break
        except Exception:
            logger.debug("Trying to connect to feature server")
            time.sleep(3)

    return AsyncRPCClient(stub)
