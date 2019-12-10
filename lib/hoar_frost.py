from datetime import datetime, timezone, timedelta
from uuid import getnode
from os import getpid


class HoarFrostGenerator:

    def __init__(self):
        # discord's epoch
        self.epoch = datetime(2015, 1, 1, tzinfo=timezone.utc)
        self.increment = 0

    def _calculate_timestamp(self):
        return (datetime.now(timezone.utc) - self.epoch) // timedelta(milliseconds=1)

    def generate(self):
        hoar_frost = self._calculate_timestamp() << 22
        hoar_frost |= (getnode() & 0b11111) << 17
        hoar_frost |= (getpid() & 0b11111) << 12
        hoar_frost |= self.increment & 0b111111111111
        self.increment += 1
        return hoar_frost
