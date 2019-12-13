import string
from contextlib import suppress
from typing import List


class ResponseMode:

    REGEX = 'regex'
    PUNCTUATED = 'punctuated'
    NAIVE = 'naive'


class AutoResponse:

    def __init__(
        self,
        bot,
        trigger,
        response,
        author_id,
        guild_id,
        id=None,
        trigger_regex=None,
        trigger_punctuation=(),
        response_ast=None,
        mode=None,
        count=0
    ):
        self.bot = bot
        self.trigger = trigger
        self.response = response
        self.author_id = author_id
        self.guild_id = guild_id
        self.count = count

        self.id = id or bot.hoar_frost_generator.generate()
        self.response_ast = response_ast or self._parse_response()
        self.mode = mode or self._determine_mode()

        if self.mode == ResponseMode.PUNCTUATED and trigger_punctuation == ():
            self.trigger_punctuation = self._extract_punctuation()
        else:
            self.trigger_punctuation = trigger_punctuation

        self.trigger_regex = trigger_regex or self._generate_trigger_regex()

    def validate(self, settings: Setting, response_list: List[AutoResponse] = ()):
        if len(self.trigger) < settings.resp_min_trigger_len:
            raise ShortTriggerException(f"Trigger is under minimum length ({settings.resp_min_trigger_len})")
        if len(self.response) > settings.resp_max_resp_len:
            raise LongResponseException(f"Response is over maximum length ({settings.resp_max_resp_len})")

        if any(intersects(self.trigger_regex, other.trigger_regex) for other in response_list):
            raise TriggerCollisionException("Trigger collides with another already set")

    def _generate_trigger_regex(self):
        # TODO
        if self.mode in ResponseMode.REGEX:
            self.trigger_regex = self.trigger
        elif self.mode == ResponseMode.PUNCTUATED:
            # escape regex stuff
            # trim whitespace
            pass
        else:
            # escape regex stuff
            # filter punction
            # trim whitespace
            pass

    def _extract_punctuation(self):
        return tuple(c for c in self.trigger if c in string.punctuation)

    def _parse_response(self):
        # TODO
        pass

    def _determine_mode(self):
        with suppress(IndexError):
            if self.trigger[0] == '^' and self.trigger[-1] == '$':
                return 'regex'

        if any(c for c in self.trigger if c in string.punctuation):
            return 'punctuated'

        return 'naive'


class TriggerCollisionException(Exception):
    pass


class ShortTriggerException(Exception):
    pass
