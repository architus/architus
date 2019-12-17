
defaults = {
    r'\w': CharClass("0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz"),
    r'\d': CharClass("0123456789"),
    r'\s': CharClass("\t\n\v\f\r "),
}
defaults[r'\W'] = ~defaults[r'\w']
defaults[r'\D'] = ~defaults[r'\d']
defaults[r'\S'] = ~defaults[r'\s']


class CharClass:

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
        return {default_char} | self.chars


