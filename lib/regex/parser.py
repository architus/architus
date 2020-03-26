import re
from math import inf
from contextlib import suppress


class CharClass:
    # short = {
    #     r'\w': CharClass("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz"),
    #     r'\d': CharClass("0123456789"),
    #     r'\s': CharClass("\t\n\v\f\r "),
    # }
    # short[r'\W'] = ~short[r'\w']
    # short[r'\D'] = ~short[r'\d']
    # short[r'\S'] = ~short[r'\s']

    special = set('\\[]|().?*+{}')
    class_special = set('\\[]^-')

    def __init__(self, chars):
        self.chars = frozenset(chars)

    def __eq__(self, o):
        return self.chars == o.chars

    def __hash__(self):
        return hash(self.chars)

    @property
    def alphabet(self):
        pass
        # return {default_char} | self.chars


class Quantifier:

    # short = {
    #     '*': Quantifier(0, inf),
    #     '+': Quantifier(1, inf),
    #     '?': Quantifier(0, 1),
    # }

    patterns = [
        re.compile(r'\{(\d),(\d)?\}'),
        re.compile(r'\{(\d)\}'),
        re.compile(r'\*'),
        re.compile(r'\+'),
        re.compile(r'\?')
    ]

    def __init__(self, min, max):
        try:
            self.min = int(min)
            self.max = int(max)
        except OverflowError:
            self.max = inf
        except ValueError:
            raise TypeError("Bounds must be of type 'int' or 'math.inf'") from None

        assert max >= min

        self.optional = max - min

    def __eq__(self, o):
        with suppress(AttributeError):
            return self.min == o.min and self.max == o.max
        return False

    def __hash__(self):
        return hash((self.min, self.max))

    def __repr__(self):
        if self.min == self.max:
            return f"{{self.min}}"
        return '{' + str(self.min) + ', ' + str(self.max) + '}'

    def _multiplicible(self, o):
        # idk how this works, but I think it's true
        return o.optional == 0 or self.optional * o.min + 1 >= self.min

    def __mul__(self, o):
        if not self._multiplicible(o):
            raise ValueError(f"{self} and {o} are not multiplicible")
        return self.__class__(self.min * o.min, self.max * o.max)

    def __add__(self, o):
        return self.__class__(self.min + o.min, self.max + o.max)

    def __sub__(self, o):
        return self.__class__(self.min - o.min, self.max - o.max)

    def __and__(self, o):
        if self.max < o.min or o.max < self.min:
            raise ValueError(f"Can't calculate intersection of {self} and {o})")
        return self.__class__(max(self.min, o.min), min(self.max, o.max))

    def __or__(self, o):
        if self.max + 1 < o.min or o.max + 1 < self.min:
            raise ValueError(f"Can't calculate union of {self} and {o})")
        return self.__class__(min(self.min, o.min), max(self.max, o.max))

    @classmethod
    def match(cls, string, i=0):
        # TODO make sure this returns the right indices

        for j, pattern in enumerate(cls.patterns):
            match = pattern.match(string, i)
            if match:
                if j == 0:  # {2,3} or {2,}
                    try:
                        return cls(match[1], match[2]), match.end
                    except IndexError:
                        return cls(match[1], inf), match.end
                elif j == 1:  # {2}
                    return cls(match[1], match[1]), match.end
                elif j == 2:  # *
                    return cls(0, inf), match.end
                elif j == 3:  # +
                    return cls(1, inf), match.end
                elif j == 4:  # ?
                    return cls(0, 1), match.end

        return cls(1, 1), i
