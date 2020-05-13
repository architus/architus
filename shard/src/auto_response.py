import string
from contextlib import suppress
from typing import Optional, Tuple
from random import choice

from lib.reggy.reggy import Reggy
from lib.response_grammar.response import parse as parse_response
from lib.config import logger


class ResponseMode:

    REGEX = 'regex'
    PUNCTUATED = 'punctuated'
    NAIVE = 'naive'


class AutoResponse:

    def __init__(
        self,
        bot,
        trigger: str,
        response: str,
        author_id: int,
        guild_id: int,
        id: Optional[int] = None,
        trigger_regex: str = "",
        trigger_punctuation: Tuple[str, ...] = (),
        response_ast: str = "",
        mode: Optional[ResponseMode] = None,
        count: int = 0
    ):
        self.bot = bot
        self.trigger = trigger
        self.response = response
        self.author_id = author_id
        self.guild_id = guild_id
        self.count = count

        if id is None:
            self.id = bot.hoarfrost_gen.generate()
        else:
            self.id = id

        if mode is None:
            self.mode = self._determine_mode()
        else:
            self.mode = mode

        if response_ast == "":
            self.response_ast = self._parse_response()
        else:
            self.response_ast = response_ast

        if self.mode == ResponseMode.PUNCTUATED and trigger_punctuation == ():
            self.trigger_punctuation = self._extract_punctuation()
        else:
            self.trigger_punctuation = trigger_punctuation

        if trigger_regex == "":
            self.trigger_regex = self._generate_trigger_regex()
        else:
            self.trigger_regex = trigger_regex

        self.trigger_reggy = Reggy(self.trigger_regex)

    def _parse_response(self):
        """parse the response into its ast"""
        return parse_response(self.response)

    def _extract_punctuation(self) -> Tuple[str, ...]:
        return tuple(c for c in self.trigger if c in string.punctuation)

    def _determine_mode(self) -> ResponseMode:
        """determine the mode of an AutoResponse based on a trigger string"""
        with suppress(IndexError):
            if self.trigger[0] == '^' and self.trigger[-1] == '$':
                return ResponseMode.REGEX

        if any(c for c in self.trigger if c in string.punctuation):
            return ResponseMode.PUNCTUATED

        return ResponseMode.NAIVE

    def _generate_trigger_regex(self) -> str:
        special_chars = ['\\', '.', '*', '+', '?', '[', ']', '(', ')']
        pattern = self.trigger

        if self.mode == ResponseMode.REGEX:
            pattern = pattern[1:-1]
        elif self.mode == ResponseMode.PUNCTUATED:
            for c in string.punctuation:
                if c not in self.trigger_punctuation:
                    pattern = pattern.replace(c, "")
            for c in special_chars:
                pattern = pattern.replace(c, f"\\{c}")
        elif self.mode == ResponseMode.NAIVE:
            for c in string.punctuation:
                pattern = pattern.replace(c, "")
            for c in special_chars:
                pattern = pattern.replace(c, f"\\{c}")
        else:
            raise AutoResponseException(f"Unsupported mode: {self.mode}")

        return pattern

        # fsm = FSM(self.trigger_regex)
        # if any(fsm.intersects(FSM(other.trigger_regex)) for other in guild_responses):
        # raise TriggerCollisionException

    def resolve_token(self, token):
        if token[0] == 'list':
            return self.resolve_token(choice(token[1]))
        elif token[0] == 'text':
            return token[1]
        elif token[0] == 'react':
            return
        elif token[0] == 'noun':
            return
        elif token[0] == 'adj':
            return
        elif token[0] == 'adv':
            return
        elif token[0] == 'count':
            return str(self.count)

    async def execute(self, msg):
        # match = self.trigger_reggy.fullmatch(msg.content)
        match = None
        if match is None:
            return False

        for token in self.response_ast:
            pass

    def __repr__(self):
        return f"<{self.trigger}::{self.response}> MODE: '{self.mode}' COUNT: '{self.count}'"


class GuildAutoResponses:

    def __init__(self, bot, guild):
        self.guild = guild
        self.bot = bot
        self.settings = self.bot.settings[guild]
        self.auto_responses = []

    async def execute(self, msg):
        for r in self.auto_responses:
            if await r.execute(msg):
                break

    def new(self, trigger, response, guild, author):
        r = AutoResponse(self.bot, trigger, response, guild.id, author.id)
        self.validate(r)
        self.auto_responses.append(r)
        return r

    def validate(self, response: AutoResponse) -> None:
        if self.settings.responses_limit is not None:
            author_count = len([r for r in self.auto_responses if r.author_id == self.author_id])
            if author_count >= self.settings.responses_limit:
                raise UserLimitException

        if len(response.response) > self.settings.responses_response_length:
            raise LongResponseException

        if len(response.trigger) < self.settings.responses_trigger_length:
            raise ShortTriggerException

        if not self.is_disjoint(response):
            raise TriggerCollisionException

    def is_disjoint(self, response: AutoResponse) -> bool:
        # all(r.trigger_reggy.isdisjoint(response.trigger_reggy) for r in self.auto_responses)
        for r in self.auto_responses:
            logger.debug(f"checking {r.trigger_reggy} against {response.trigger_reggy} {r.trigger_reggy.isdisjoint(response.trigger_reggy)}")
            if not r.trigger_reggy.isdisjoint(response.trigger_reggy):
                logger.debug(f"{response} collides with {r}")
                return False
        return True


class AutoResponseException(Exception):
    pass


class ShortTriggerException(AutoResponseException):
    pass


class LongResponseException(AutoResponseException):
    pass


class UserLimitException(AutoResponseException):
    pass


class TriggerCollisionException(AutoResponseException):
    pass
