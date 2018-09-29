import json
from src.models import Settings
from sqlalchemy.orm.exc import NoResultFound

class server_settings:

    def __init__(self, session, server_id):
        self.session = session
        self.server_id = server_id
        self._settings_dict = self._load_from_db(server_id)


    @property
    def default_role_id(self) -> str:
        return self._settings_dict['default_role'] if 'default_role' in self._settings_dict else ''

    @default_role_id.setter
    def default_role_id(self, new_id: str):
        self._settings_dict['default_role'] = new_id
        self._update_db()

    @property
    def bot_commands_channels(self) -> list:
        return self._settings_dict['bot_commands'] if 'bot_commands' in self._settings_dict else []

    @bot_commands_channels.setter
    def bot_commands_channels(self, new_bot_commands: list):
        print (new_bot_commands)
        self._settings_dict['bot_commands'] = new_bot_commands
        self._update_db()

    @property
    def admins_ids(self) -> list:
        return self._settings_dict['admins'] if 'admins' in self._settings_dict else []

    @admins_ids.setter
    def admins_ids(self, new_admins: list):
        self._settings_dict['admins'] = new_admins
        self._update_db()

    @property
    def emojis(self) -> dict:
        return self._settings_dict['emojis'] if 'emojis' in self._settings_dict else {}

    @emojis.setter
    def emojis(self, new_emojis: dict):
        self._settings_dict['emojis'] = new_emojis
        self._update_db()

    def _load_from_db(self, server_id) -> dict:
        settings_row = None
        try:
            settings_row = self.session.query(Settings).filter_by(server_id = int(server_id)).one()
        except NoResultFound as e:
            new_server = Settings(int(self.server_id), json.dumps({}))
            self.session.add(new_server)
        return json.loads(settings_row.json_blob) if settings_row else {}

    def _update_db(self):
        new_data = {
            'server_id' : int(self.server_id),
            'json_blob' : json.dumps(self._settings_dict)
        }
        self.session.query(Settings).filter_by(server_id = int(self.server_id)).update(new_data)
        self.session.commit()


