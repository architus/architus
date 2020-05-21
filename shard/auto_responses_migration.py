import os
import string
import time
from sqlalchemy.orm import sessionmaker
from sqlalchemy import create_engine
import json
from collections import defaultdict

from lib.models import Command, AutoResponse
from lib.hoar_frost import HoarFrostGenerator
from lib.response_grammar.response import parse
from lib.reggy.reggy import Reggy

from src.auto_response import ResponseMode

hoarfrost_gen = HoarFrostGenerator()

DB_HOST = 'postgres'
DB_PORT = 5432
db_user = os.environ['db_user']
db_pass = os.environ['db_pass']

engine = create_engine(f"postgresql://{db_user}:{db_pass}@{DB_HOST}:{DB_PORT}/autbot")
Session = sessionmaker(bind=engine)
session = Session()

command_list = session.query(Command).all()


def get_puncutation(trigger):
    return tuple(c for c in trigger if c in string.punctuation)


def get_mode(trigger):
    if '*' in trigger:
        return ResponseMode.REGEX
    if any(c for c in trigger if c in string.punctuation):
        return ResponseMode.PUNCTUATED
    return ResponseMode.NAIVE


def generate_trigger(old_trigger, guild_id):
    trigger = old_trigger.replace(f"{cmd.server_id}", "", 1)
    if '*' in trigger:
        trigger = f"^{trigger}$"
    return trigger


def generate_trigger_regex(trigger, mode):
    special_chars = ['\\', '.', '*', '+', '?', '[', ']', '(', ')', '|']
    other_special_chars = ['\\', '.', '+', '?', '[', ']', '(', ')', '|']
    pattern = trigger.lower()

    if mode == ResponseMode.REGEX:
        pattern = pattern[1:-1]
        for c in other_special_chars:
            pattern = pattern.replace(c, f"\\{c}")
        pattern = " ?".join(pattern)
        pattern = pattern.replace("*", "(.*)")
    elif mode == ResponseMode.PUNCTUATED:
        for c in string.punctuation:
            if c not in get_puncutation(trigger):
                pattern = pattern.replace(c, "")
        for c in special_chars:
            pattern = pattern.replace(c, f"\\{c}")
        pattern = " ?".join(pattern)
    elif mode == ResponseMode.NAIVE:
        for c in string.punctuation:
            pattern = pattern.replace(c, "")
        for c in special_chars:
            pattern = pattern.replace(c, f"\\{c}")
        pattern = " ?".join(pattern)
    else:
        raise Exception(f"Unsupported mode: {mode}")

    return pattern


def generate_response(response):
    return response.replace("[capture]", "[0]")


def generate_response_ast(response):
    response = generate_response(response)
    try:
        return json.dumps(parse(response).stringify())
    except Exception as e:
        print(e.__class__.__name__)
        print(f"caught while parsing: '{response}'")
        return ""


responses = defaultdict(list)
for cmd in command_list:
    trigger = generate_trigger(cmd.trigger, cmd.server_id)
    responses[cmd.server_id].append(AutoResponse(
        hoarfrost_gen.generate(),
        trigger,
        generate_response(cmd.response),
        cmd.author_id,
        cmd.server_id,
        generate_trigger_regex(trigger, get_mode(trigger)),
        get_puncutation(trigger),
        generate_response_ast(cmd.response),
        get_mode(trigger),
        cmd.count
    ))

for guild_id in responses.keys():
    reggys = []
    for resp in responses[guild_id]:
        try:
            reggys.append(Reggy(resp.trigger_regex))
        except Exception as e:
            print(e.__class__.__name__)
            print(f"caught while compiling '{resp.trigger_regex}'")

    print(f"scanning {guild_id} for collisions...")
    now = time.time()
    count = 0
    for i in range(len(reggys)):
        for j in range(len(reggys)):
            if i == j:
                continue
            if not reggys[i].isdisjoint(reggys[j]):
                print(f"{reggys[i]} intersects {reggys[j]}")
                count += 1
    print(f"found collisions in {count}/{len(reggys)} triggers in {time.time() - now} seconds")

i = input("scan complete, do you want to insert (y/n)?")
if i.lower() != 'y':
    exit("exiting...")

try:
    for guild_id in responses.keys():
        for r in responses[guild_id]:
            session.add(r)
except Exception:
    session.rollback()
    raise
else:
    session.commit()
    print("migration successful!")
