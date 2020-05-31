import datetime


class Base:

    def __init__(self, conn_wrapper):
        self.conn_wrapper = conn_wrapper

    @property
    def conn(self):
        return self.conn_wrapper.conn

    async def insert(self, cols):
        columns, values = zip(*cols.items())
        print(f'''INSERT INTO {self.__class__.__tablename__}({','.join(columns)})
            VALUES ({','.join(str(item) for item in values)})
            ''')
        await self.conn.execute(
            f'''INSERT INTO {self.__class__.__tablename__}({','.join(columns)})
            VALUES ({','.join(str(item) for item in values)})
            '''
        )

    async def update_by_id(self, cols, id):
        assigns = (f"{c} = ${n + 2}" for n, c in enumerate(cols.keys()))
        await self.conn.execute(
            f'''UPDATE {self.__class__.__tablename__} SET
            {','.join(assigns)}
            WHERE id = $1
            ''', id, *cols.values()
        )


class TbAutoResponses(Base):
    __tablename__ = 'tb_auto_responses'

class TbReactEvents(Base):
    __tablename__ = 'tb_react_events'

    async def insert(self, message_id: int, guild_id: int, channel_id: int, command: int, payload: str, expires_on: datetime.datetime):
        cols = {
            'message_id': message_id,
            'guild_id': guild_id,
            'channel_id': channel_id,
            'command': command,
            'payload': payload,
            'expires_on': expires_on
        }

        await super().insert(cols)


    async def get_by_id(self, message_id:int, guild_id: int):
        return await self.conn.fetchrow(
            f'''SELECT *
            FROM {self.__class__.__tablename__}
            WHERE (guild_id, message_id) = ($1, $2)
            ''', guild_id, message_id
        )

