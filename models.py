from sqlalchemy.ext.declarative import declarative_base
from sqlalchemy import Column, ForeignKey, Integer, BigInteger, Float
from config import session

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
