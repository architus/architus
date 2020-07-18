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

    async def insert_one(self, values):
        async with (await self.pool()).acquire() as conn:
            async with conn.transaction():
                await conn.execute(
                    f'''INSERT INTO {self.__class__.__tablename__} VALUES
                    ({','.join(f'${n + 1}' for n in range(len(values)))})
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


class TbAutoResponses(Base):
    __tablename__ = 'tb_auto_responses'


class TbEmojis(Base):
    __tablename__ = 'tb_emojis'
