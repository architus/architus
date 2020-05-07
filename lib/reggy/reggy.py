import fsm
from functools import reduce
import json
import re2 as re


class NotParseable(Exception):
    pass


class ABCReggy:
    """
    Abstract base class for all of the regex stuff.
    """

    def to_fsm(self, alphabet):
        """
        Get the FSM that represents the regular expression.
        """
        raise NotImplementedError

    def __repr__(self):
        raise NotImplementedError

    @classmethod
    def match(cls, string, i=0):
        """
        Match given string[i:] to given class.
        """
        raise NotImplementedError

    @classmethod
    def parse(cls, string):
        """
        Parse the string as an instance of the given class.
        """
        obj, i = cls.match(string, 0)
        if i != len(string):
            raise Exception("Could not parse entire string.")
        return obj

    def __reversed__(self):
        return self.reversed()

    def __str__(self):
        return self.__repr__()


class Bound:
    """
    Represents a bound in a regular expression.
    Such as one of the numbers in {3,10}.
    Can possibly be infinite (None).
    """

    def __init__(self, b):
        if b is None:
            self.bound = b
            self.infinite = True
            return
        if b < 0:
            raise Exception(f"Invalid bound: {b}")
        else:
            self.bound = b
            self.infinite = False

    def __repr__(self):
        if self.infinite:
            return "Inf"
        else:
            return f"{self.bound}"

    def __str__(self):
        return self.__repr__()

    @classmethod
    def match(cls, string, i=0):
        b = 0
        found = False
        # First try matching for integers
        try:
            while i < len(string):
                v = int(string[i])
                b *= 10
                b += v
                i += 1
                found = True
        except ValueError:
            pass

        if found:
            return cls(b), i
        else:
            return cls(None), i

    def __eq__(self, other):
        if isinstance(other, Bound):
            return (self.bound == other.bound and
                    self.infinite == other.infinite)
        else:
            return False

    def __hash__(self):
        return hash(self.bound)

    def __lt__(self, other):
        if not isinstance(other, Bound):
            return False
        elif self.infinite:
            return False
        elif other.infinite:
            return True
        else:
            return self.bound < other.bound

    def __gt__(self, other):
        if not isinstance(other, Bound):
            return False
        if self.infinite:
            if other.infinite:
                return False
            return True
        if other.infinite:
            return False
        return self.bound > other.bound

    def __ge__(self, other):
        return not self < other

    def __mul__(self, other):
        assert isinstance(other, Bound)
        if self.bound == 0 or other.bound == 0:
            return Bound(0)
        if self.infinite or other.infinite:
            return Bound(None)
        return Bound(self.bound, other.bound)

    def __add__(self, other):
        assert isinstance(other, Bound)
        if self.infinite or other.infinite:
            return Bound(None)
        return Bound(self.bound, other.bound)

    def __sub__(self, other):
        """
        This is kinda sketchy.
        """
        assert isinstance(other, Bound)
        if other.infinite:
            if not self.infinite:
                raise Exception("Invalid operation")
            return Bound(0)
        if self.infinite:
            return self
        return Bound(self.bound - other.bound)

    def copy(self):
        return Bound(self.bound)


class Multiplier:
    """
    A set of bounds such as the entirety of {3,10}.
    """

    def __init__(self, minimum, maximum):
        if minimum.infinite:
            raise Exception("Minimum bound can't be infinite")
        if minimum > maximum:
            raise Exception("Min can't be larger than max")

        self.minimum = minimum
        self.maximum = maximum
        self.mandatory = self.minimum
        self.optional = self.maximum - self.minimum

    def __eq__(self, other):
        if not isinstance(other, Multiplier):
            return False
        return (self.minimum == other.minimum and
                self.maximum == other.maximum)

    def __hash__(self):
        return hash((self.minimum, self.maximum))

    def __repr__(self):
        return "{" + str(self.minimum) + "," + str(self.maximum) + "}"

    def __str__(self):
        return self.__repr__()

    @classmethod
    def match(cls, string, i=0):
        # If at end of string just return {1, 1}
        if i >= len(string):
            return cls(Bound(1), Bound(1)), i
        # First check if multiplier is a special character
        elif string[i] == "?":
            if i + 1 < len(string) and string[i + 1] in {"*", "?", "+"}:
                raise NotParseable
            return cls(Bound(0), Bound(1)), i + 1
        elif string[i] == "*":
            if i + 1 < len(string) and string[i + 1] in {"*", "?", "+"}:
                raise NotParseable
            return cls(Bound(0), Bound(None)), i + 1
        elif string[i] == "+":
            if i + 1 < len(string) and string[i + 1] in {"*", "?", "+"}:
                raise NotParseable
            return cls(Bound(1), Bound(None)), i + 1

        # Check if curly brackets
        if string[i] == "{":
            lower, j = Bound.match(string, i + 1)

            # If now at end brackets, require specifically that many
            if string[j] == "}":
                if j + 1 < len(string) and string[j + 1] in {"*", "?", "+"}:
                    raise NotParseable
                return cls(lower, lower), j + 1
            # Then string[j] must be a comma.
            # If string[j + 1] is a } then it's lower - inf
            if string[j + 1] == "}":
                if i + 1 < len(string) and string[i + 1] in {"*", "?", "+"}:
                    raise NotParseable
                return cls(lower, Bound(None)), j + 2

            # Last case is to parse the second part of the {}
            upper, j = Bound.match(string, j + 1)
            if j < len(string) and string[j] in {"*", "?", "+"}:
                raise NotParseable
            return cls(lower, upper), j + 1

        # if none of those things were found then return {1,1}
        return cls(Bound(1), Bound(1)), i

    @classmethod
    def parse(cls, string):
        """
        Attempt to parse the entire string as an instance of the class.
        Mainly used for unit testing.
        """
        obj, i = cls.match(string, 0)
        if i != len(string):
            raise Exception("Could not parse whole string.")
        return obj

    def canmultiply(self, other):
        """
        Range multiplication is wacky.
        """
        return (other.optional == Bound(0) or
                self.optional * other.mandatory + Bound(1) >= self.mandatory)

    def __mul__(self, other):
        if not self.canmultiply(other):
            raise Exception("These ranges can't multiply together")
        return Multiplier(self.minimum * other.minimum,
                          self.maximum * other.maximum)

    def __add__(self, other):
        return Multiplier(self.minimum + other.minimum,
                          self.maximum + other.maximum)

    def __sub__(self, other):
        """
        Warning. Not well defined for all pairs of ranges.
        """
        mandatory = self.mandatory - other.mandatory
        optional = self.optional - other.optional
        return Multiplier(mandatory, mandatory + optional)

    def canintersect(self, other):
        return not (self.maximum < other.minimum or
                    other.maximum < self.minimum)

    def __and__(self, other):
        if not self.canintersect(other):
            raise Exception("Can't intersect these two ranges.")
        a = max(self.minimum, other.minimum)
        b = min(self.maximum, other.maximum)
        return Multiplier(a, b)

    def canunion(self, other):
        return not (self.maximum + one < other.minimum or
                    other.maximum + one < self.minimum)

    def __or__(self, other):
        if not self.canunion(other):
            raise Exception("Can't union these ranges.")
        a = min(self.minimum, other.minimum)
        b = max(self.maximum, other.maximum)
        return Multiplier(a, b)

    def common(self, other):
        mandatory = min(self.mandatory, other.mandatory)
        optional = min(self.optional, other.optional)
        return Multiplier(mandatory, mandatory + optional)

    def copy(self):
        return Multiplier(self.minimum.copy(), self.maximum.copy())


zero = Multiplier(Bound(0), Bound(0))
qm = Multiplier(Bound(0), Bound(1))
one = Multiplier(Bound(1), Bound(1))
star = Multiplier(Bound(0), Bound(None))
plus = Multiplier(Bound(1), Bound(None))


class Conc(ABCReggy):
    """
    Concatenation of Mults.
    """

    def __init__(self, *mults):
        self.mults = tuple(mults)

    def __eq__(self, other):
        if not isinstance(other, Conc):
            return False
        return self.mults == other.mults

    def __hash__(self):
        return hash(self.mults)

    def __repr__(self):
        string = "conc("
        string += "\n".join(repr(m) for m in self.mults)
        string += ")"
        return string

    def times(self, multiplier):
        if multiplier == one:
            return self
        return Pattern(self) * multiplier

    def concatenate(self, other):
        if isinstance(other, Pattern) or isinstance(other, CharacterClass):
            other = Mult(other, one)
        if isinstance(other, Mult):
            other = Conc(other)

        return Conc(*(self.mults + other.mults))

    def union(self, other):
        return Pattern(self) | other

    def intersection(self, other):
        return Pattern(self) & other

    def to_fsm(self, alphabet=None):
        if alphabet is None:
            alphabet = self.alphabet()

        fsm1 = fsm.epsilon(alphabet)
        for m in self.mults:
            fsm1 += m.to_fsm(alphabet)
        return fsm1

    def alphabet(self):
        return {fsm.unspecified}.union(*[m.alphabet() for m in self.mults])

    def empty(self):
        for m in self.mults:
            if m.empty():
                return True
        return False

    def __str__(self):
        return self.__repr__()

    @classmethod
    def match(cls, string, i=0):
        mults = list()

        m, i = Mult.match(string, i)
        while m is not None and i < len(string) and string[i] != "|":
            mults.append(m)
            m, i = Mult.match(string, i)

        if m is not None:
            mults.append(m)

        if len(mults) == 0:
            return None, i
        return Conc(*mults), i


class Mult(ABCReggy):
    """
    Combination of character matching and a multiplier.
    """

    def __init__(self, multiplicand, multiplier):
        self.multiplicand = multiplicand
        self.multiplier = multiplier

    def __eq__(self, other):
        if not isinstance(other, Mult):
            return False
        return (self.multiplicand == other.multiplicand and
                other.multiplier == other.multiplier)

    def __hash__(self):
        return hash((self.multiplicand, self.multiplier))

    def __repr__(self):
        string = "mult("
        string += repr(self.multiplicand)
        string += "\n" + repr(self.multiplier)
        string += ")"
        return string

    def times(self, multiplier):
        if multiplier == one:
            return self
        if self.multiplier.canmultiply(multiplier):
            return Mult(self.multiplicand, self.multiplier * multiplier)
        return Mult(Pattern(Conc(self)), multiplier)

    def concatenate(self, other):
        return Conc(self) + other

    def union(self, other):
        return Conc(self) | other

    def dock(self, other):
        if other.multiplicand != self.multiplicand:
            raise Exception("Can't subtract different multiplicands.")
        return Mult(self.multiplicand, self.multiplier - other.multiplier)

    def common(self, other):
        if self.multiplicand == other.multiplicand:
            return Mult(self.multiplicand,
                        self.multiplier.common(other.multiplier))

        return Mult(nothing, zero)

    def intersection(self, other):
        if isinstance(other, CharacterClass):
            other = Mult(other, one)

        if isinstance(other, Mult):
            if (self.multiplicand == other.multiplicand and
                    self.canintersect(other)):
                return Mult(self.multiplicand,
                            self.multiplier & other.multiplier)

        return Conc(self) & other

    def empty(self):
        return self.multiplicand.empty() and self.multiplier.minimum > Bound(0)

    def to_fsm(self, alphabet=None):
        if alphabet is None:
            alphabet = self.alphabet()

        unit = self.multiplicand.to_fsm(alphabet)
        mandatory = unit * self.multiplier.mandatory.bound

        if self.multiplier.optional == Bound(None):
            optional = unit.star()
        else:
            optional = fsm.epsilon(alphabet) | unit
            optional *= self.multiplier.optional.bound

        return (mandatory + optional)

    @classmethod
    def match(cls, string, i=0):
        if i == len(string):
            return None, i
        elif string[i] == "(":
            multiplicand, j = Pattern.match(string, i + 1)
            assert string[j] == ")"
            j += 1
        else:
            multiplicand, j = CharacterClass.match(string, i)
            if multiplicand is None:
                return None, i
        multiplier, j = Multiplier.match(string, j)
        return cls(multiplicand, multiplier), j

    def reversed(self):
        return Mult(reversed(self.multiplicand), self.multiplier)

    def copy(self):
        return Mult(self.multiplicand.copy(), self.muliplier.copy())

    def alphabet(self):
        return {fsm.unspecified} | self.multiplicand.alphabet()


class CharacterClass(ABCReggy):
    """
    A frozenset of symbols. Used to represent the various regular expression
    character classes.
    """

    def __init__(self, chars=set(), negated=False):
        self.chars = frozenset(chars)
        self.negated = negated

        if fsm.unspecified in self.chars:
            raise Exception("Can't have unspecified in charclass.")

    def __eq__(self, other):
        if isinstance(other, CharacterClass):
            return self.chars == other.chars and self.negated == other.negated
        else:
            return False

    def __hash__(self):
        return hash((self.chars, self.negated))

    def times(self, multiplier):
        if multiplier == one:
            return self
        return Mult(self, multiplier)

    special1 = set(r"\[]|().?*+{}-")
    special2 = set(r"\[]^-")

    def __str__(self):
        if self in shorthand.keys():
            return shorthand[self]

        if self.negated:
            return "[^" + self.escape() + "]"

        if len(self.chars) == 1:
            char = "".join(self.chars)

            if char in escapes.keys():
                return escapes[char]

            if char in CharacterClass.special1:
                return "\\" + char

            return char

        return "[" + self.escape() + "]"

    def __repr__(self):
        string = ", ".join([c for c in self.chars])
        string += ", " + repr(self.negated)
        return string

    def escape(self):
        char_range = set()
        for char in self.chars:
            if char in CharacterClass.special1:
                char_range.add("\\"+char)
            elif char in escapes.keys():
                char_range.add(escapes[char])
            else:
                char_range.add(char)
        return "".join([c for c in char_range])

    def to_fsm(self, alphabet=None):
        if alphabet is None:
            alphabet = self.alphabet()

        if self.negated:
            transition = {
                0: dict([(symbol, 1) for symbol in (alphabet - self.chars)])
            }
        else:
            transition = {
                0: dict([(symbol, 1) for symbol in self.chars])
            }

        return fsm.FSM(
            alphabet=alphabet,
            states={0, 1},
            initial=0,
            accepting={1},
            transition=transition
        )

    def concatenate(self, other):
        return Mult(self, one) + other

    def alphabet(self):
        return {fsm.unspecified} | self.chars

    def empty(self):
        return len(self.chars) == 0 and not self.negated

    def match_wildcard(string, i=0):
        for k in shorthand.keys():
            if i + len(shorthand[k]) <= len(string):
                if string[i:i + len(shorthand[k])] == shorthand[k]:
                    return k, i + len(shorthand[k])
        return None, i

    def match_unit(string, i=0, brackets=False, start=-1):
        cc, j = CharacterClass.match_wildcard(string, i)
        if cc is not None:
            return cc, j
        elif string[i] == "|" and brackets:
            return "|", i + 1
        elif string[i] == "|" and not brackets:
            return None, i
        elif string[i] == '-' and not brackets:
            return '-', i + 1
        elif string[i] == '-' and string[i + 1] == ']':
            return '-', i + 1
        elif string[i] == '-' and i - 1 == start:
            return '0', i + 1
        elif string[i] == '-':
            s = ord(string[i - 1]) + 1
            e = ord(string[i + 1]) + 1
            if e < s:
                raise NotParseable
            chars = ""
            for v in range(s, e):
                chars += chr(v)
            return chars, i + 2
        elif string[i] == '\\':
            if string[i + 1] not in CharacterClass.special1:
                raise NotParseable
            return string[i + 1], i + 2
        elif string[i] in CharacterClass.special1 and not brackets:
            return None, i
        else:
            return string[i], i + 1

    def match_bracket(string, start, end):
        i = start
        negated = False
        if string[i] == "^":
            negated = True
            i += 1

        chars = []
        while i < end:
            c, i = CharacterClass.match_unit(string, i, True, start)
            chars.append(c)

        if None in chars:
            return None, start
        cc = reduce(lambda x, y: x.union(y),
                    map(lambda x: x if isinstance(x, CharacterClass)
                        else CharacterClass(x), chars),
                    CharacterClass())
        cc.negated = negated
        return cc, end + 1

    @classmethod
    def match(cls, string, i=0):
        if i >= len(string):
            raise NotParseable

        cc, j = CharacterClass.match_wildcard(string, i)
        if cc is not None:
            return cc, j

        if string[i] == "[":
            end = string.find("]", i)
            if end == -1:
                raise NotParseable
            return CharacterClass.match_bracket(string, i + 1, end)

        c, i = CharacterClass.match_unit(string, i)
        if c is None:
            return None, i
        if not isinstance(c, CharacterClass):
            c = CharacterClass(c)
        return c, i

    def negate(self):
        return CharacterClass(self.chars, negated=(not self.negated))

    def __invert__(self):
        return self.negate()

    def union(self, other):
        if self.negated:
            if other.negated:
                return ~CharacterClass(self.chars & other.chars)
            return ~CharacterClass(self.chars - other.chars)
        if other.negated:
            return ~CharacterClass(other.chars - self.chars)
        return CharacterClass(self.chars | other.chars)

    def __or__(self, other):
        return self.union(other)

    def intersection(self, other):
        if not isinstance(other, CharacterClass):
            return Mult(self, one) & other
        if self.negated:
            if other.negated:
                return ~CharacterClass(self.chars | other.chars)
            return CharacterClass(other.chars - self.chars)
        if other.negated:
            return CharacterClass(self.chars - other.chars)
        return CharacterClass(self.chars & other.chars)

    def __and__(self, other):
        return self.intersection(other)

    def reversed(self):
        return self

    def copy(self):
        return CharacterClass(self.chars.copy(), negated=self.negated)


nothing = CharacterClass()
w = CharacterClass("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_"
                   "abcdefghijklmnopqrtsuvwxyz")
d = CharacterClass("0123456789")
s = CharacterClass("\t\n\v\f\r ")
W = ~w
D = ~d
S = ~s
dot = ~CharacterClass()

shorthand = {
    w: r"\w",
    d: r"\d",
    s: r"\s",
    W: r"\W",
    D: r"\D",
    S: r"\S",
    dot: "."
}

escapes = {
    "\t": r"\t",
    "\n": r"\n",
    "\v": r"\v",
    "\f": r"\f",
    "\r": r"\r"
}


class Pattern(ABCReggy):

    def __init__(self, *concs):
        if all([c is None for c in concs]):
            self.concs = frozenset()
        else:
            self.concs = frozenset(concs)

    def __eq__(self, other):
        if not isinstance(other, Pattern):
            return False
        return self.concs == other.concs

    def __hash__(self):
        return hash(self.concs)

    def __repr__(self):
        string = "pattern("
        string += "\n".join(repr(c) for c in self.concs)
        string += ")"
        return string

    def times(self, multiplier):
        if multiplier == one:
            return self
        return Mult(self, multiplier)

    def concatenate(self, other):
        return Mult(self, one) + other

    def alphabet(self):
        return {fsm.unspecified}.union(*[c.alphabet() for c in self.concs])

    def empty(self):
        for c in self.concs:
            if not c.empty():
                return False
        return True

    def intersection(self, other):
        alphabet = self.alphabet() | other.alphabet()
        return self.to_fsm(alphabet) & other.to_fsm(alphabet)

    def union(self, other):
        if isinstance(other, CharacterClass):
            other = Mult(other, one)
        if isinstance(other, Mult):
            other = Conc(other)
        if isinstance(other, Conc):
            other = Pattern(other)

        return Pattern(*(self.concs | other.concs))

    def __str__(self):
        return self.__repr__()

    @classmethod
    def match(cls, string, i=0):
        concs = list()

        c, i = Conc.match(string, i)
        concs.append(c)

        while c is not None and i < len(string):
            if string[i] == ")":
                return Pattern(*concs), i
            if string[i] == "|":
                c, i = Conc.match(string, i + 1)
                concs.append(c)

        if i == len(string) and c is not None:
            concs.append(c)

        return Pattern(*concs), i

    def to_fsm(self, alphabet=None):
        if alphabet is None:
            alphabet = self.alphabet()

        return reduce(
            lambda x, y: x | y,
            map(
                lambda x: x.to_fsm(alphabet),
                self.concs),
            fsm.null(alphabet)
        )

    def equivalent(self, other):
        return self.to_fsm().equivalent(other.to_fsm())


def serialize(obj):
    if isinstance(obj, fsm.UnspecifiedCharacter):
        return obj.__dict__

    raise TypeError


def as_unspecified(dct):
    if "__unspecified__" in dct:
        return fsm.unspecified
    return dct


class Reggy():
    """
    Actual regex class that will use the reggy backend.
    """

    def __init__(self, regex, rep=None):
        if rep is not None:
            self.re = rep['re']
            self.fsm = fsm.FSM(
                rep['alphabet'],
                rep['states'],
                rep['initial'],
                rep['accepting'],
                rep['transition'],
                __validation__=False
            )
            self.pattern = re.compile(self.re)
            return
        if regex.find("[[") != -1:
            raise NotParseable
        if regex.find("\\p") != -1:
            raise NotParseable
        if regex.find("(?") != -1:
            raise NotParseable
        self.re = regex
        self.fsm = Pattern.parse(self.re).to_fsm().reduce()
        self.pattern = re.compile(self.re)

    def __str__(self):
        return self.re

    def __repr__(self):
        return self.re

    def matches(self, string):
        return self.pattern.fullmatch(string)

    def accepts(self, string):
        return self.pattern.fullmatch(string)

    def __contains__(self, string):
        return self.fsm.accepts(string)

    def isdisjoint(self, other):
        if not isinstance(other, Reggy):
            raise TypeError
        return self.fsm.isdisjoint(other.fsm)

    def to_json(self):
        return json.dumps({
            're': self.re,
            'alphabet': list(self.fsm.alphabet),
            'states': list(self.fsm.states),
            'initial': self.fsm.initial,
            'accepting': list(self.fsm.accepting),
            'transition': self.fsm.transition
        }, default=serialize)

    @classmethod
    def from_json(cls, data):
        rep = json.loads(data, object_hook=as_unspecified)
        return cls("", rep)
