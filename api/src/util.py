import os
from uuid import getnode
from functools import wraps
from datetime import datetime, timedelta

from flask_restful import Resource, reqparse
from flask import g

from lib.config import get_session, which_shard
from lib.auth import JWT
from lib.ipc.blocking_rpc_client import get_rpc_client


class ShardClientWrapper:
    def __init__(self):
        self.topic = (getnode() << 15) | os.getpid()
        self.client = get_rpc_client(self.topic)

    def __getattr__(self, name):
        def call(*args, routing_guild=None, **kwargs):
            return self.client.call(name, *args, routing_key=f"shard_rpc_{which_shard(routing_guild)}", **kwargs)
        return call


class CustomResource(Resource):
    '''Default flask Resource but contains tools to talk to the shard nodes and the db.'''
    def __init__(self):
        self.shard = ShardClientWrapper()

    @property
    def session(self):
        if 'db' not in g:
            g.db = get_session()
        return g.db


def reqparams(**params):
    def decorator(func):
        @wraps(func)
        def wrapper(*args, **kwargs):
            parser = reqparse.RequestParser()
            for param, type in params.items():
                parser.add_argument(param, type=type, required=True)
            values = parser.parse_args()
            kwargs.update(values)
            return func(*args, **kwargs)
        return wrapper
    return decorator


def camelcase_keys(dictionary: dict):
    for key in list(dictionary.keys()):
        first, *rest = key.split('_')
        new_key = first + ''.join(word.capitalize() for word in rest)
        dictionary[new_key] = dictionary.pop(key)
    return dictionary


def str_keys_recursive(dictionary):
    try:
        for k, v in dictionary.items():
            dictionary[str(k)] = str_keys_recursive(v)
            del dictionary[k]
    except AttributeError:
        pass
    return dictionary


def time_to_refresh(jwt: JWT):
    issued_at = datetime.strptime(jwt.issued_at, "%Y-%m-%dT%H:%M:%S.%f")
    refresh_in = timedelta(seconds=jwt.expires_in) / 2
    return datetime.now() > issued_at + refresh_in
