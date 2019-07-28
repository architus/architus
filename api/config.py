import yaml
from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

DB_HOST = 'postgres'
DB_PORT = 5432

try:
    with open('.secrets.yaml') as f:
        data = yaml.safe_load(f)
except FileNotFoundError:
    with open('../.secrets.yaml') as f:
        data = yaml.safe_load(f)

client_id = data['client_id']
client_secret = data['client_secret']
db_user = data['db_user']
db_pass = data['db_pass']


def get_session():
    engine = create_engine(f"postgresql://{db_user}:{db_pass}@{DB_HOST}:{DB_PORT}/autbot")
    Session = sessionmaker(bind=engine)
    return Session()
