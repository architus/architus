from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

secret_token = None
db_user = None
db_pass = None

try:
    lines = [line.rstrip('\n') for line in open('.secret_token')]
    secret_token = lines[0]
    db_user = lines[1]
    db_pass = lines[2]

except Exception as e:
    print('error reading .secret_token, make it you aut')

try:
    engine = create_engine("postgresql://{}:{}@localhost/autbot".format(db_user, db_pass))
    Session = sessionmaker(bind=engine)
    session = Session()

except Exception as e:
    print('failed to connect to database')
    print(e)
