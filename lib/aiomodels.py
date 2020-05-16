

class Base:

    def __init__(self, conn_wrapper):
        self.conn_wrapper = conn_wrapper

    @property
    def conn(self):
        return self.conn_wrapper.conn

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
