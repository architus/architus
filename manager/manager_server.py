import os
from datetime import datetime, timedelta
from concurrent.futures import ThreadPoolExecutor
import threading
import time

from lib.config import logger, domain_name
from lib.hoar_frost import HoarFrostGenerator

import grpc
import lib.ipc.manager_pb2_grpc as manager_grpc
import lib.ipc.manager_pb2 as message
from lib.ipc.grpc_client import grpc_options


class Manager(manager_grpc.ManagerServicer):
    """
    Implements a server for the Manager gRPC protocol.
    """

    def __init__(self, total_shards):
        """
        Instantiates a new manager server that handles some
        number of shards.
        """
        logger.info(f"Number of shards: {total_shards}")
        self.hoarfrost_gen = HoarFrostGenerator()
        self.total_shards = total_shards
        self.registered = [False for _ in range(total_shards)]
        self.last_checkin = dict()
        self.store = dict()

    def health_check(self):
        while True:
            time.sleep(1)
            for shard, last_checkin in self.last_checkin.items():
                if last_checkin is not None and last_checkin < datetime.now() - timedelta(seconds=1.5):
                    logger.error(f"--- SHARD {shard} MISSED ITS HEARTBEAT, DEREGISTERING... ---")
                    self.registered[shard] = False
                    self.last_checkin[shard] = None

    def register(self, request, context):
        """Returns the next shard id that needs to be filled as well as the total shards"""
        if all(self.registered):
            raise Exception("Shard trying to register even though we're full")
        i = next(i for i in range(self.total_shards) if not self.registered[i])
        logger.info(f"Shard requested id, assigning {i + 1}/{self.total_shards}...")
        self.registered[i] = True
        # Give the bot some seconds to get set up before expecting heartbeats.
        self.last_checkin[i] = datetime.now() + timedelta(seconds=5)
        return message.ShardInfo(shard_id=i, shard_count=self.total_shards)

    def guild_count(self, request, context):
        """Return guild and user count information"""
        gc = 0
        uc = 0
        for guilds in self.store.values():
            gc += len(guilds)
            for guild in guilds:
                uc += guild.member_count

        return message.GuildInfo(guild_count=gc, user_count=uc)

    def checkin(self, request, context):
        self.last_checkin[request.shard_id] = datetime.now()
        self.registered[request.shard_id] = True
        return message.CheckInResponse()

    def publish_file(self, request_iterator, context):
        """Missing associated documentation comment in .proto file"""
        first = next(request_iterator)
        filetype = "png" if first.filetype == "" else first.filetype
        name = first.name
        if name == "":
            name = str(self.hoarfrost_gen.generate())
        location = first.location
        if location == "":
            location = "assets"
        directory = f"/var/www/{location}"

        if not os.path.exists(directory):
            os.makedirs(directory)
        with open(f"{directory}/{name}.{filetype}", "wb") as f:
            logger.info(f"Writing {directory}/{name}.{filetype}")
            f.write(first.file)
            for datum in request_iterator:
                f.write(datum.file)

        return message.Url(url=f"https://cdn.{domain_name}/{location}/{name}.{filetype}")

    def all_guilds(self, request, context):
        """Return information about all guilds that the bot is in, including their admins"""
        for guilds in self.store.values():
            for guild in guilds:
                yield guild

    def guild_update(self, request_iterator, context):
        """Update the manager with the latest information about a shard's guilds"""
        guilds = []
        for guild in request_iterator:
            guilds.append(guild)
        if len(guilds) == 0:
            return message.UpdateResponse()
        logger.debug(f"Received guild list from shard {guilds[0].shard_id + 1} of {len(guilds)} guilds")
        self.store[guilds[0].shard_id] = guilds
        return message.UpdateResponse()


def serve(manager):
    server = grpc.server(ThreadPoolExecutor(max_workers=20), options=grpc_options)
    manager_grpc.add_ManagerServicer_to_server(manager, server)
    server.add_insecure_port("0.0.0.0:50051")
    server.start()
    logger.debug("gRPC server started")
    server.wait_for_termination()


if __name__ == "__main__":
    manager = Manager(int(os.environ["NUM_SHARDS"]))
    health = threading.Thread(target=manager.health_check, daemon=True)
    health.start()
    serve(manager)
