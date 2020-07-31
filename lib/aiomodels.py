from asyncio import sleep


class Base:

    def __init__(self, conn_wrapper):
        self.conn_wrapper = conn_wrapper

    async def pool(self):
        while self.conn_wrapper.pool is None:
            await sleep(0.1)
        return self.conn_wrapper.pool

    async def select_all(self):
        async with (await self.pool()).acquire() as conn:
            return await conn.fetch(f'SELECT * FROM {self.__class__.__tablename__}')

    async def select_by_guild(self, guild_id):
        async with (await self.pool()).acquire() as conn:
            return await conn.fetch(f'SELECT * FROM {self.__class__.__tablename__} WHERE guild_id = $1', guild_id)

    async def insert_one(self, values):
        async with (await self.pool()).acquire() as conn:
            async with conn.transaction():
                await conn.execute(
                    f'''INSERT INTO {self.__class__.__tablename__} VALUES
                    ({','.join(f'${n + 1}' for n in range(len(values)))})
                    ''', *values
                )

    async def insert(self, cols):
        columns, values = zip(*cols.items())
        async with (await self.pool()).acquire() as conn:
            async with conn.transaction():
                await conn.execute(
                    f'''INSERT INTO {self.__class__.__tablename__}({','.join(columns)})
                    VALUES ({','.join(f'${num}' for num in range(1, len(values) + 1))})
                    ''', *values
                )

    async def update_by_id(self, cols, id):
        assigns = (f"{c} = ${n + 2}" for n, c in enumerate(cols.keys()))
        async with (await self.pool()).acquire() as conn:
            async with conn.transaction():
                await conn.execute(
                    f'''UPDATE {self.__class__.__tablename__} SET
                    {','.join(assigns)}
                    WHERE id = $1
                    ''', id, *cols.values()
                )

    async def delete_by_id(self, id):
        async with (await self.pool()).acquire() as conn:
            async with conn.transaction():
                await conn.execute(f'DELETE FROM {self.__class__.__tablename__} WHERE id = $1', id)

    async def select_by_id(self, cols):
        columns, values = zip(*cols.items())
        async with (await self.pool()).acquire() as conn:
            async with conn.transaction():
                return await conn.fetchrow(
                    f'''SELECT *
                    FROM {self.__class__.__tablename__}
                    WHERE ({','.join(columns)}) = ({','.join(f'${num}' for num in range(1, len(values) + 1))})
                    ''', *cols.values()
                )


class TbAutoResponses(Base):
    __tablename__ = 'tb_auto_responses'


class TbReactEvents(Base):
    __tablename__ = 'tb_react_events'

    async def insert(self, message_id: int, guild_id: int, channel_id: int,
                     event_type: int, payload: str):
        cols = {
            'message_id': message_id,
            'guild_id': guild_id,
            'channel_id': channel_id,
            'event_type': event_type,
            'payload': payload
        }

        await super().insert(cols)

    async def select_by_id(self, message_id: int, guild_id: int):
        cols = {
            'message_id': message_id,
            'guild_id': guild_id
        }
        return await super().select_by_id(cols)


class TbEmojis(Base):
    __tablename__ = 'tb_emojis'

    async def select_by_guild(self, guild_id):
        async with (await self.pool()).acquire() as conn:
            return await conn.fetch(
                f'SELECT * FROM {self.__class__.__tablename__} WHERE guild_id = $1 ORDER BY priority', guild_id)
