from sqlalchemy import create_engine
from sqlalchemy.orm import sessionmaker

from src.commands.quote_command import quote_command
from src.commands.set_command import set_command
from src.commands.spectrum_command import spectrum_command
from src.commands.role_command import role_command
from src.commands.gulag_command import gulag_command
from src.commands.play_command import play_command
from src.commands.schedule_command import schedule_command

secret_token = None
db_user = None
db_pass = None

try:
    lines = [line.rstrip('\n') for line in open('.secret_token')]
    secret_token = lines[0]
    db_user = lines[1]
    db_pass = lines[2]

except Exception as e:
    print(e)
    print('error reading .secret_token, make it you aut')

try:
    engine = create_engine("postgresql://{}:{}@localhost/autbot".format(db_user, db_pass))
    Session = sessionmaker(bind=engine)
    session = Session()

except Exception as e:
    print('failed to connect to database')
    print(e)

default_cmds = {
        'quote' : quote_command(),
        'set' : set_command(),
        'role' : role_command(),
        'play' : play_command(),
        'gulag' : gulag_command(),
        'spectrum' : spectrum_command(),
        'schedule' : schedule_command()
    }
