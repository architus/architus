import string
from contextlib import suppress


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

        self.id = id or bot.hoar_frost_gen.generate()
        self.response_ast = response_ast or self._parse_response()
        self.mode = mode or self._determine_mode()

        if self.mode == ResponseMode.PUNCTUATED and trigger_punctuation == ():
            self.trigger_punctuation = self._extract_punctuation()
        else:
            self.trigger_punctuation = trigger_punctuation

        self.trigger_regex = trigger_regex or self._generate_trigger_regex()

    def _generate_trigger_regex(self):
        # TODO
        if self.mode == ResponseMode.REGEX:
            self.trigger_regex = self.trigger
        elif self.mode == ResponseMode.PUNCTUATED:
            pass
        else:
            pass

        if self._collision_detector():
            raise TriggerCollisionException()

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

    def validate(self, bot, ctx):
        settings = bot.settings[ctx.guild]
        guild_responses = bot.autoresponses[ctx.guild]

        author_count = len([_ for r in guild_responses if r.author_id == self.author_id])
        if settings.responses_limit is not None and author_count >= settings.responses_limit:
            raise UserLimitException

       if len(self.response) > settings.responses_response_length:
           raise LongResponseException

       if len(self.trigger) < settings.responses_trigger_length:
           raise ShortTriggerException

       fsm = FSM(self.trigger_regex)
       if any(fsm.intersects(FSM(other.trigger_regex)) for other in guild_responses):
           raise TriggerCollisionException

    async def triggered(self, msg):
        pass


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
