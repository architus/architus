from discord import Emoji

from PIL import ImageChops, Image
from lib.hoar_frost import HoarFrostGenerator
from src.utils import download_emoji
from lib.config import logger


hoarfrost_gen = HoarFrostGenerator()


class ArchitusEmoji:

    @classmethod
    async def from_discord(cls, emoji: Emoji):
        '''creates an architus emoji from a discord emoji'''
        im = Image.open(await download_emoji(emoji))
        return cls(im, emoji.name, None, emoji.id)

    def __init__(
            self,
            im: Image,
            name: str,
            id: int = None,
            discord_id: int = None,
            num_uses: int = 0,
            priority: float = 0.0):

        self.im = im
        self.name = name

        if id is None:
            self.id = hoarfrost_gen.generate()
        else:
            self.id = id

        self.discord_id = discord_id
        self.num_uses = num_uses
        self.priority = priority

    @property
    def loaded(self):
        return self.discord_id is not None

    def cache(self):
        self.discord_id = None

    def update(self, o):
        self.name = o.name
        self.discord_id = o.discord_id

    def update_from_discord(self, e: Emoji):
        self.name = e.name
        self.discord_id = e.id

    def _im_eq(self, o):
        '''tell if two emojis have the same image'''
        try:
            return ImageChops.difference(self.im, o.im).getbbox() is None
        except ValueError:
            logger.debug("VALUE ERROR IN IM_EQ")
            return False

    def __eq__(self, o):
        return self.id == o.id or \
            self.discord_id == o.discord_id or \
            (self._im_eq(o) and self.name == o.name)

    def __hash__(self):
        return hash(self.id)

    def __repr__(self):
        return f"<:{self.name}:{self.id}>"
