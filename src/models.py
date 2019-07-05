from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy import Column, Integer, BigInteger, Float, Text, DateTime

Base = declarative_base()


class User(Base):
    __tablename__ = 'tb_users'
    discord_id = Column('discord_id', BigInteger, primary_key=True)
    aut_score = Column('aut_score', Float)
    norm_score = Column('norm_score', Float)
    nice_score = Column('nice_score', Float)
    toxic_score = Column('toxic_score', Float)
    awareness_score = Column('awareness_score', Integer)

    def __init__(self, discord_id, scores):
        self.discord_id = discord_id
        self.aut_score = scores[0]
        self.norm_score = scores[1]
        self.nice_score = scores[2]
        self.toxic_score = scores[3]
        self.toxic_score = scores[4]

    def as_dict(self):
        return {
            'discord_id': self.discord_id,
            'scores': [self.aut_score, self.norm_score, self.nice_score, self.toxic_score, self.awareness_score]
        }

    def as_entry(self):
        return [self.aut_score, self.norm_score, self.nice_score, self.toxic_score, self.awareness_score]


class Admin(Base):
    __tablename__ = 'tb_admins'
    discord_id = Column('discord_id', BigInteger, primary_key=True)
    server_id = Column('server_id', BigInteger)
    username = Column('username', Text)

    def __init__(self, server_id, discord_id, username):
        self.server_id = server_id
        self.discord_id = discord_id
        self.username = username


class Settings(Base):
    __tablename__ = 'tb_settings'
    server_id = Column('server_id', BigInteger, primary_key=True)
    json_blob = Column('json_blob', Text)

    def __init__(self, server_id, json_blob):
        self.server_id = server_id
        self.json_blob = json_blob


class AppSession(Base):
    __tablename__ = 'tb_session'
    autbot_access_token = Column('autbot_access_token', Text, primary_key=True)
    discord_access_token = Column('discord_access_token', Text)
    discord_refresh_token = Column('discord_refresh_token', Text)
    discord_expiration = Column('discord_expiration', DateTime)
    autbot_expiration = Column('autbot_expiration', DateTime)
    last_login = Column('last_login', DateTime)
    discord_id = Column('discord_id', BigInteger)

    def __init__(self, autbot_access_token, discord_access_token,
                 discord_refresh_token, discord_expiration, autbot_expiration, discord_id, last_login=None):

        self.autbot_access_token = autbot_access_token
        self.discord_access_token = discord_access_token
        self.discord_refresh_token = discord_refresh_token
        self.discord_expiration = discord_expiration
        self.autbot_expiration = autbot_expiration
        self.last_login = last_login
        self.discord_id = discord_id


class Log(Base):
    __tablename__ = 'tb_logs'
    guild_id = Column('guild_id', BigInteger, primary_key=True)
    type = Column('type', Text, primary_key=True)
    message_id = Column('message_id', BigInteger)
    content = Column('content', Text, primary_key=True)

    def __init__(self, guild_id, type, content, message_id=None):
        self.guild_id = guild_id
        self.type = type
        self.content = content
        self.message_id = message_id


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
