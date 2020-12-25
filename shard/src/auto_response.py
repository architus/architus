import string
from contextlib import suppress
from typing import Optional, Tuple
from random import choice
import json

from discord import Message, Guild, Member, AllowedMentions

from src.emoji_manager import EmojiManager
from lib.reggy.reggy import Reggy
from lib.response_grammar.response import parse as parse_response, NodeType
from lib.config import logger
from lib.aiomodels import TbAutoResponses


class WordGen:
    def __init__(self):
        with open("res/words/nouns.txt") as f:
            self.nouns = list(set(noun.strip() for noun in f))
        with open("res/words/adjectives.txt") as f:
            self.adjs = list(set(adj.strip() for adj in f))
        with open("res/words/adverbs.txt") as f:
            self.advs = list(set(adv.strip() for adv in f))

    @property
    def noun(self):
        return choice(self.nouns)

    @property
    def adj(self):
        return choice(self.adjs)

    @property
    def adv(self):
        return choice(self.advs)


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
        count: int = 0,
        word_gen: Optional[WordGen] = None,
        emoji_manager: Optional[EmojiManager] = None
    ):
        self.bot = bot
        self.trigger = trigger
        self.response = response
        self.author_id = author_id
        self.guild_id = guild_id
        self.count = count
        self.word_gen = word_gen

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

        self._emoji_manager = emoji_manager
        self.trigger_reggy = Reggy(self.trigger_regex)
        self.not_trigger_punctuation = "".join([c for c in string.punctuation if c not in self.trigger_punctuation])

    @property
    def emoji_manager(self):
        if self._emoji_manager is None:
            self._emoji_manager = self.bot.get_cog("Emoji Manager").managers[self.guild_id]
        return self._emoji_manager

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
        special_chars = ['\\', '.', '*', '+', '?', '[', ']', '(', ')', '|']
        pattern = self.trigger.lower()

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

    async def resolve_resp(self, node, match, msg, content=None, reacts=None):
        if (node.type == NodeType.List):
            await self.resolve_resp(choice(node.children), match, msg, content, reacts)
        elif (node.type == NodeType.ListElement):
            for c in node.children:
                await self.resolve_resp(c, match, msg, content, reacts)
        elif (node.type == NodeType.PlainText):
            content.append(node.text)
        elif (node.type == NodeType.React):
            emoji = self.emoji_manager.find_emoji(node.id, node.id, node.shortcode)
            if emoji:
                logger.debug(f"found {emoji} in manager, making sure it's loaded")
                await self.emoji_manager.load_emoji(emoji)
                reacts.append(emoji.to_discord_str())
            else:
                reacts.append(node.shortcode)
        elif (node.type == NodeType.Noun):
            content.append(self.word_gen.noun)
        elif (node.type == NodeType.Adj):
            content.append(self.word_gen.adj)
        elif (node.type == NodeType.Adv):
            content.append(self.word_gen.adv)
        elif (node.type == NodeType.Count):
            content.append(str(self.count))
        elif (node.type == NodeType.Member):
            content.append(choice(msg.guild.members).display_name)
        elif (node.type == NodeType.Author):
            content.append(msg.author.display_name)
        elif (node.type == NodeType.Capture):
            with suppress(IndexError):
                string = match.groups()[node.capture_group]
                content.append(string if string is not None else "")

        elif (node.type == NodeType.Url):
            content.append(node.text)
        else:
            content = []
            reacts = []
            for c in node.children:
                await self.resolve_resp(c, match, msg, content, reacts)
            return content, reacts

    async def execute(self, msg: Message) -> Optional[Message]:
        content = msg.content

        if self.mode == ResponseMode.REGEX:
            pass
        else:
            content = content.translate(str.maketrans('', '', self.not_trigger_punctuation))

        match = self.trigger_reggy.matches(content)
        if match is None:
            return

        self.count += 1
        content, reacts = await self.resolve_resp(self.response_ast, match, msg)
        content = "".join(content)

        if content.strip() != "":
            resp_msg = await msg.channel.send(content, allowed_mentions=AllowedMentions(everyone=False))
        else:
            resp_msg = None
        for emoji in reacts:
            logger.debug(f"trying to react: {emoji}")
            await msg.add_reaction(emoji)

        return resp_msg

    def __repr__(self):
        return f"{self.trigger}::{self.response}"

    def as_dict(self):
        return {
            'trigger': self.trigger,
            'response': self.response,
            'authorId': str(self.author_id) if self.author_id else None,
            'guildId': str(self.guild_id),
            'id': str(self.id),
            'triggerRegex': self.trigger_regex,
            'triggerPunctuation': self.trigger_punctuation,
            'responseTokens': self.response_ast.stringify(),
            'mode': self.mode,
            'count': self.count,
        }


class GuildAutoResponses:

    def __init__(self, bot, guild, executor, no_db=False):
        self.guild = guild
        self.bot = bot
        self.executor = executor
        self.tb_auto_responses = TbAutoResponses(self.bot.asyncpg_wrapper)
        self.settings = self.bot.settings[guild]
        self.auto_responses = []
        self.word_gen = WordGen()
        self.no_db = no_db

    @classmethod
    async def new(cls, *args, **kwargs):
        guild_auto_responses = cls(*args, **kwargs)
        await guild_auto_responses._init_from_db()
        return guild_auto_responses

    @property
    def aiosession(self):
        return self.bot.aiosession

    async def _init_from_db(self) -> None:
        if self.no_db:
            return

        for r in await self.tb_auto_responses.select_by_guild(self.guild.id):
            try:
                args = (
                    self.bot,
                    r['trigger'],
                    r['response'],
                    r['author_id'] if r['author_id'] != 0 else None,
                    r['guild_id'],
                    r['id'],
                    r['trigger_regex'],
                    r['trigger_punctuation'],
                    "",
                    r['mode'],
                    r['count'],
                    self.word_gen,
                    None)  # TODO
                resp = await self.bot.loop.run_in_executor(self.executor, AutoResponse, *args)
            except Exception:
                logger.exception("")
            else:
                self.auto_responses.append(resp)

    async def _insert_into_db(self, resp: AutoResponse) -> None:
        if self.no_db:
            return
        await self.tb_auto_responses.insert_one((
            resp.id,
            resp.trigger,
            resp.response,
            resp.author_id,
            resp.guild_id,
            resp.trigger_regex,
            resp.trigger_punctuation,
            json.dumps(resp.response_ast.stringify()),
            resp.mode,
            resp.count
        ))

    async def _delete_from_db(self, resp: AutoResponse) -> None:
        if self.no_db:
            return
        await self.tb_auto_responses.delete_by_id(resp.id)

    async def _update_resp_db(self, resp: AutoResponse) -> None:
        if self.no_db:
            return
        await self.tb_auto_responses.update_by_id({'count': resp.count}, resp.id)

    async def execute(self, msg) -> Tuple[Optional[Message], Optional[AutoResponse]]:
        if msg.author.bot:
            return None, None
        for r in self.auto_responses:
            resp_msg = await r.execute(msg)
            if resp_msg is not None:
                await self._update_resp_db(r)
                return resp_msg, r
        return None, None

    async def new_response(self, trigger: str, response: str, guild: Guild, author: Member) -> AutoResponse:
        """factory method for creating a guild-specific auto response"""
        if self.no_db:
            manager = None
        else:
            manager = self.bot.get_cog("Emoji Manager").managers[guild.id]
        r = await self.bot.loop.run_in_executor(
            self.executor, AutoResponse,
            self.bot,
            trigger.strip(),
            response.strip(),
            author.id,
            guild.id,
            None,
            "",
            (),
            "",
            None,
            0,
            self.word_gen,
            manager)

        self.validate(r)
        self.auto_responses.append(r)
        await self._insert_into_db(r)
        return r

    async def remove(self, trigger: str, author: Member) -> AutoResponse:
        """helper method for removing guild-specific auto response"""
        for r in self.auto_responses:
            if r.trigger == trigger:
                admin = r.author_id in self.settings.admin_ids
                if not admin and self.settings.responses_only_author_remove and r.author_id != author.id:
                    raise PermissionException(r.author_id)
                self.auto_responses.remove(r)
                await self._delete_from_db(r)
                return r
        raise UnknownResponseException

    def validate(self, response: AutoResponse) -> None:
        admin = response.author_id in self.settings.admin_ids
        if not self.settings.responses_enabled:
            raise DisabledException("auto responses")
        if response.mode == ResponseMode.REGEX and not (self.settings.responses_allow_regex or admin):
            raise DisabledException("regex responses")

        if self.settings.responses_limit is not None and not admin:
            author_count = len([r for r in self.auto_responses if r.author_id == response.author_id])
            if author_count >= self.settings.responses_limit:
                raise UserLimitException

        if not admin and len(response.response) > self.settings.responses_response_length:
            raise LongResponseException

        if not admin and len(response.trigger) < self.settings.responses_trigger_length:
            raise ShortTriggerException

        if not self.settings.responses_allow_collision:
            conflicts = self.is_disjoint(response)
            if conflicts:
                raise TriggerCollisionException(conflicts)
        else:
            for r in self.auto_responses:
                if r.trigger == response.trigger:
                    raise TriggerCollisionException((r,))

    def is_disjoint(self, response: AutoResponse) -> bool:
        # all(r.trigger_reggy.isdisjoint(response.trigger_reggy) for r in self.auto_responses)
        conflicts = []
        for r in self.auto_responses:
            if not r.trigger_reggy.isdisjoint(response.trigger_reggy):
                conflicts.append(r)
        return conflicts


class AutoResponseException(Exception):
    pass


class PermissionException(AutoResponseException):
    def __init__(self, author_id):
        self.author_id = author_id


class DisabledException(AutoResponseException):
    pass


class ShortTriggerException(AutoResponseException):
    pass


class LongResponseException(AutoResponseException):
    pass


class UserLimitException(AutoResponseException):
    pass


class UnknownResponseException(AutoResponseException):
    pass


class TriggerCollisionException(AutoResponseException):
    def __init__(self, conflicts):
        self.conflicts = conflicts
