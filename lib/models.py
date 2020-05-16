from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy import Column, Integer, BigInteger, Float, Text, DateTime, LargeBinary
from sqlalchemy.dialects.postgresql import VARCHAR, ARRAY, CHAR


Base = declarative_base()


class Emoji(Base):

    __tablename__ = 'tb_emojis'
    id = Column('id', BigInteger, primary_key=True)
    name = Column('name', Text)
    discord_id = Column('discord_id', BigInteger)
    author_id = Column('author_id', BigInteger)
    guild_id = Column('guild_id', BigInteger)
    num_uses = Column('num_uses', Integer)
    priority = Column('priority', Float)
    img = Column('img', LargeBinary)

    def __init__(self, id, discord_id, author_id, guild_id, name, num_uses, priority, img):
        self.id = id
        self.discord_id = discord_id
        self.author_id = author_id
        self.guild_id = guild_id
        self.name = name
        self.num_uses = num_uses
        self.priority = priority
        self.img = img


class AutoResponse(Base):

    __tablename__ = 'tb_auto_responses'
    id = Column('id', BigInteger, primary_key=True)
    trigger = Column('trigger', Text)
    response = Column('response', Text)
    author_id = Column('author_id', BigInteger)
    guild_id = Column('guild_id', BigInteger)
    trigger_regex = Column('trigger_regex', Text)
    trigger_punctuation = Column('trigger_punctuation', ARRAY(CHAR))
    response_ast = Column('response_ast', Text)
    mode = Column('mode', VARCHAR(10))
    count = Column('count', Integer)

    def __init__(self, id, trigger, response, author_id, guild_id,
                 trigger_regex, trigger_punctuation, response_ast, mode, count):
        self.id = id
        self.trigger = trigger
        self.response = response
        self.author_id = author_id
        self.guild_id = guild_id
        self.trigger_regex = trigger_regex
        self.trigger_punctuation = trigger_punctuation
        self.response_ast = response_ast
        self.mode = mode
        self.count = count


class Settings(Base):
    __tablename__ = 'tb_settings'
    server_id = Column('server_id', BigInteger, primary_key=True)
    json_blob = Column('json_blob', Text)

    def __init__(self, server_id, json_blob):
        self.server_id = server_id
        self.json_blob = json_blob


class Log(Base):
    __tablename__ = 'tb_logs'
    guild_id = Column('guild_id', BigInteger, primary_key=True)
    type = Column('type', Text, primary_key=True)
    message_id = Column('message_id', BigInteger)
    user_id = Column('user_id', BigInteger)
    content = Column('content', Text, primary_key=True)
    timestamp = Column('timestamp', DateTime)

    def __init__(self, guild_id, type, content, user_id, message_id, timestamp):
        self.guild_id = guild_id
        self.type = type
        self.content = content
        self.message_id = message_id
        self.timestamp = timestamp
        self.user_id = user_id


class Command(Base):
    __tablename__ = 'tb_commands'
    trigger = Column('trigger', Text, primary_key=True)
    response = Column('response', Text)
    count = Column('count', BigInteger)
    server_id = Column('server_id', BigInteger, primary_key=True)
    author_id = Column('author_id', BigInteger)

    def __init__(self, trigger, response, count, server_id, author_id):
        self.trigger = trigger
        self.response = response
        self.count = count
        self.server_id = server_id
        self.author_id = author_id
