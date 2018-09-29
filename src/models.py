from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy import Column, ForeignKey, Integer, BigInteger, Float, Text

Base = declarative_base()

class User(Base):
    __tablename__ = 'tb_users'
    discord_id = Column('discord_id', BigInteger, primary_key=True)
    aut_score = Column('aut_score', Float)
    norm_score = Column('norm_score', Float)
    nice_score = Column('nice_score', Float)
    toxic_score = Column('toxic_score', Float)

    def __init__(self, discord_id, scores):
        self.discord_id = discord_id
        self.aut_score = scores[0]
        self.norm_score = scores[1]
        self.nice_score = scores[2]
        self.toxic_score = scores[3]
    
    def as_dict(self):
        return {
            'discord_id' : self.discord_id,
            'scores' : [self.aut_score, self.norm_score, self.nice_score, self.toxic_score]
        }
    
    def as_entry(self):
        return [self.aut_score, self.norm_score, self.nice_score, self.toxic_score]

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
    
class Role(Base):
    __tablename__ = 'tb_roles'
    target_role_id = Column('target_role_id', BigInteger, primary_key=True)
    server_id = Column('server_id', BigInteger)
    required_role_id = Column('required_role_id', BigInteger)

    def __init__(self, server_id, target_role_id, required_role_id):
        self.server_id = server_id
        self.target_role_id = target_role_id
        self.required_role_id = required_role_id

class Command(Base):
    __tablename__ = 'tb_commands'
    trigger = Column('trigger', Text, primary_key=True)
    response = Column('response', Text)
    count = Column('count', BigInteger)
    server_id = Column('server_id', BigInteger)
    author_id = Column('author_id', BigInteger)

    def __init__(self, trigger, response, count, server_id, author_id):
        self.trigger = trigger
        self.response = response
        self.count = count
        self.server_id = server_id
        self.author_id = author_id
