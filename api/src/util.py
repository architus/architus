import os
from uuid import getnode

from flask_restful import Resource, reqparse
from flask import g

from lib.config import get_session, which_shard
from lib.blocking_rpc_client import get_rpc_client


class CustomResource(Resource):
    '''Default flask Resource but contains tools to talk to the shard nodes and the db.'''
    def __init__(self):
        self.topic = (getnode() << 15) | os.getpid()
        self.client = get_rpc_client(self.topic)

    @property
    def session(self):
        if 'db' not in g:
            g.db = get_session()
        return g.db

    def shard_call(self, method, *args, routing_guild=None, **kwargs):
        '''Queues an RPC request to a shard.'''
        return self.client.call(
            method,
            *args,
            routing_key=f"shard_rpc_{which_shard(routing_guild)}",
            **kwargs
        )


def reqparams(**params):
    def decorator(func):
        def wrapper(*args, **kwargs):
            parser = reqparse.RequestParser()
            for param, type in params:
                parser.add_argument(param, type=type, required=True)
            values = parser.parse_args()
            kwargs.update(values)
            func(*args, **kwargs)
        return wrapper
    return decorator
