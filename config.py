from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

secret_token = None

try:
    lines = [line.rstrip('\n') for line in open('.secret_token')]
    secret_token = lines[0]

except Exception as e:
    print('error reading .secret_token, make it you aut')

try:
    engine = create_engine("postgresql://matt:password@localhost/autbot")
    Session = sessionmaker(bind=engine)
    session = Session()

except Exception as e:
    print('failed to connect to database')
    print(e)
