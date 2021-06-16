from typing import Optional

from PIL import ImageChops, Image
from discord import Emoji

from lib.hoar_frost import HoarFrostGenerator
from src.utils import download_emoji
from lib.config import logger, domain_name


hoarfrost_gen = HoarFrostGenerator()


class ArchitusEmoji:

    @classmethod
    async def from_discord(cls, bot, emoji: Emoji):
        '''creates an architus emoji from a discord emoji'''
        im = Image.open(await download_emoji(emoji))
        return cls(bot, im, emoji.name, None, emoji.id, emoji.user.id if emoji.user is not None else None)

    def __init__(
            self,
            bot,
            im: Image,
            name: str,
            id: Optional[int] = None,
            discord_id: Optional[int] = None,
            author_id: Optional[int] = None,
            num_uses: int = 0,
            priority: float = 0.0):

        self.bot = bot
        self.im = im
        self.name = name

        if id is None:
            self.id = hoarfrost_gen.generate()
        else:
            self.id = id

        self.author_id = author_id
        self.discord_id = discord_id
        self.num_uses = num_uses
        self.priority = priority

    @property
    def loaded(self):
        return self.discord_id is not None

    @property
    def url(self):
        if self.loaded:
            return f"https://cdn.discordapp.com/emojis/{self.discord_id}.png"
        return f"https://cdn.{domain_name}/emojis/{self.id}"

    def cache(self) -> None:
        self.discord_id = None

    def update(self, o):
        self.name = o.name
        self.discord_id = o.discord_id
        self._url = o.url
        return self

    def update_from_discord(self, e: Emoji):
        self.name = e.name
        self.discord_id = e.id
        if e.user is not None:
            self.author_id = e.user.id
        return self

    def to_discord_str(self):
        logger.debug(f"to_discord_str: <:{self.name}:{self.discord_id}>")
        return f"<:{self.name}:{self.discord_id}>"

    def _im_eq(self, o):
        '''tell if two emojis have the same image'''
        try:
            return ImageChops.difference(self.im, o.im).getbbox() is None
        except ValueError:
            # logger.debug("VALUE ERROR IN IM_EQ")
            return False

    def __eq__(self, o):
        return self.id == o.id or \
            self.discord_id == o.discord_id or \
            (self._im_eq(o) and self.name == o.name)

    def __hash__(self):
        return hash(self.id)

    def __repr__(self):
        return f"<:{self.name}:{self.id}>"

    def as_dict(self):
        return {
            'id': str(self.id),
            'name': self.name,
            'authorId': str(self.author_id),
            'discordId': str(self.discord_id) if self.discord_id else None,
            'numUses': self.num_uses,
            'priority': self.priority,
            'url': self.url,
        }

    async def as_dict_url(self):
        return {
            'id': str(self.id),
            'name': self.name,
            'authorId': str(self.author_id),
            'discordId': str(self.discord_id) if self.discord_id else None,
            'numUses': self.num_uses,
            'priority': self.priority,
            'url': self.url,
        }
