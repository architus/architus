"""
Finite state machine library
"""

from typing import Any, Set, Dict


class UnspecifiedCharacter:
    """
    Allows finite state machines to use this as a transition
    symbol that represents anything not explicitly specified
    in the input language.
    """

    def __init__(self):
        self.__unspecified__ = "unspecified"

    def __str__(self):
        return "unspecified"

    def __repr__(self):
        return "unspecified"


unspecified = UnspecifiedCharacter()


class OblivionError(Exception):
    """
    Exception to be raised when an oblivion state is reached.
    The oblivion state occurs when an unspecified transition occurs.
    """
    pass


class FSM:
    """
    The finite state machine class.
    """

    def __init__(self, alphabet: Set, states: Set, initial: int,
                 accepting: Set, transition: Dict[int, Dict[Any, int]],
                 *, __validation__=True):
        if __validation__:
            """
            Validate the specified state machine to ensure that it is valid.
            """
            if initial not in states:
                raise Exception("Initial state not in set of all states")
            if not accepting.issubset(states):
                raise Exception("Final states must be a subset of all states")
            for state in transition.keys():
                for symbol in transition[state]:
                    if transition[state][symbol] not in states:
                        raise Exception(f"Transition of {state},{symbol} "
                                        "-> {transition[state][symbol]} "
                                        "is invalid")

        self.alphabet = set(alphabet)
        self.states = set(states)
        self.initial = initial
        self.accepting = set(accepting)
        self.transition = transition

    def valid_transition(self, state: int, symbol: str) -> bool:
        """
        Checks to see if a transition is valid given the
        current state and the symbol being read.
        """
        return state in self.transition and symbol in self.transition[state]

    def accepts(self, string: str) -> bool:
        curr_state = self.initial
        for symbol in string:
            if unspecified in self.alphabet and symbol not in self.alphabet:
                symbol = unspecified

            if not (curr_state in self.transition and
                    symbol in self.transition[curr_state]):
                return False

            curr_state = self.transition[curr_state][symbol]
        return curr_state in self.accepting

    def __contains__(self, string: str) -> bool:
        """
        Allows for the syntax of `"a" in fsm` to test matching.
        """
        return self.accepts(string)

    def reduce(self):
        """
        A fun result of automota theory is that reversing a FSM
        twice will result in the same automota in reduced form.
        """
        return reversed(reversed(self))

    def __repr__(self) -> str:
        representation = []
        representation.append("FSM:\n")
        representation.append(f"\tAlphabet: {repr(self.alphabet)}\n")
        representation.append(f"\tStates: {repr(self.states)}\n")
        representation.append(f"\tInitial State: {repr(self.initial)}\n")
        representation.append(f"\tAccepting States: {repr(self.accepting)}\n")
        representation.append(f"\tTransition Function: "
                              f"{repr(self.transition)}\n")
        return "".join(representation)

    def __str__(self) -> str:
        rows = []

        # top row
        row = ["", "name", "accepting?"]
        whitespace = set(['\n', '\t', ' ', '\r', '\f'])
        row.extend(str(symbol) if symbol not in whitespace else
                   repr(symbol)[1:-1] for symbol in
                   self.alphabet)
        rows.append(row)

        # other rows
        for state in self.states:
            row = []
            if state == self.initial:
                row.append("*")
            else:
                row.append("")
            row.append(str(state))
            if state in self.accepting:
                row.append("True")
            else:
                row.append("False")
            for symbol in self.alphabet:
                if (state in self.transition and
                        symbol in self.transition[state]):
                    row.append(str(self.transition[state][symbol]))
                else:
                    row.append("")
            rows.append(row)

        # column widths
        colwidths = []
        for x in range(len(rows[0])):
            colwidths.append(max(len(str(rows[y][x]))
                                 for y in range(len(rows))) + 1)

        # apply padding
        for y in range(len(rows)):
            for x in range(len(rows[y])):
                rows[y][x] = rows[y][x].ljust(colwidths[x])

        # horizontal line
        rows.insert(1, ["-" * colwidth for colwidth in colwidths])

        return "".join("".join(row) + "\n" for row in rows)

    def concatenate(*fsms):
        """
        Concatenate arbitrarily many fsms together to form a single FSM.
        Order of fsms in arguments is assumed to be order they should
        be concatenated.
        """
        if len(fsms) == 0:
            return epsilon({})
        alphabet = set().union(*[fsm.alphabet for fsm in fsms])
        last_index, last = len(fsms) - 1, fsms[-1]

        def connect_all(i, substate):
            result = {(i, substate)}
            while i < last_index and substate in fsms[i].accepting:
                i += 1
                substate = fsms[i].initial
                result.add((i, substate))
            return result

        initial = set()
        if len(fsms) > 0:
            initial.update(connect_all(0, fsms[0].initial))
        initial = frozenset(initial)

        def accepts(state):
            """
            Test if state is accepting in last fsm
            """
            for (i, substate) in state:
                if i == last_index and substate in last.accepting:
                    return True
            return False

        def follow(current, symbol):
            next_states = set()
            for (i, substate) in current:
                fsm = fsms[i]
                if fsm.valid_transition(substate, symbol):
                    next_states.update(
                        connect_all(i, fsm.transition[substate][symbol]))
            if not next_states:
                raise OblivionError
            return frozenset(next_states)

        return crawl(alphabet, initial, accepts, follow)

    def __add__(self, other):
        return self.concatenate(other)

    def star(self):
        """
        If the current FSM accepts any string of the form R,
        then return a new FSM that accepts any string of the
        form R*. Does the kleene closure operation.
        """
        alphabet = self.alphabet
        initial = {self.initial}

        def follow(state, symbol):
            next_states = set()
            for substate in state:
                if self.valid_transition(substate, symbol):
                    next_states.add(self.transition[substate][symbol])

                if (substate in self.accepting and
                        symbol in self.transition[self.initial]):
                    next_states.add(self.transition[self.initial][symbol])

            if len(next_states) == 0:
                raise OblivionError

            return frozenset(next_states)

        def accepts(state):
            return any(substate in self.accepting for substate in state)

        base = crawl(alphabet, initial, accepts, follow)
        num_states = len(base.states)
        base.accepting.add(num_states)
        base.transition[num_states] = base.transition[base.initial]
        base.initial = num_states
        return base

    def times(self, multiplier):
        if multiplier < 0:
            raise Exception("Can't multiply by a negative number")

        if multiplier == 0:
            return epsilon(self.alphabet)

        if multiplier == 1:
            return self

        alphabet = self.alphabet
        initial = {(self.initial, 0)}

        def accepts(state):
            for (substate, iteration) in state:
                if ((substate == self.initial) and
                        (self.initial in self.accepting or
                         iteration == multiplier)):
                    return True
            return False

        def follow(current, symbol):
            next_states = set()
            for (substate, iteration) in current:
                if (iteration < multiplier and
                        self.valid_transition(substate, symbol)):
                    next_states.add((self.transition[substate][symbol],
                                     iteration))
                    if (self.transition[substate][symbol] in self.accepting):
                        next_states.add((self.initial, iteration + 1))
            if len(next_states) == 0:
                raise OblivionError
            return frozenset(next_states)

        return crawl(alphabet, initial, accepts, follow)

    def __mul__(self, multiplier):
        return self.times(multiplier)

    def union(*fsms):
        return parallel(fsms, any)

    def __or__(self, other):
        return self.union(other)

    def intersection(*fsms):
        """
        Finds the intersection of all fsms passed.
        """
        return parallel(fsms, all)

    def __and__(self, other):
        return self.intersection(other)

    def symmetric_difference(*fsms):
        return parallel(fsms, lambda accepts: (accepts.count(True) % 2) == 1)

    def __xor__(self, other):
        return self.symmetric_difference(other)

    def everythingbut(self):
        """
        Returns the FSM that is the inverse of this FSM.
        """
        alphabet = self.alphabet
        initial = {0: self.initial}

        def follow(current, symbol):
            next_states = dict()
            if (0 in current and current[0] in self.transition and
                    symbol in self.transition[current[0]]):
                next_states[0] = self.transition[current[0]][symbol]
            return next_states

        def accepts(state):
            return not (0 in state and state[0] in self.accepting)

        return crawl(alphabet, initial, accepts, follow)

    def reversed(self):
        alphabet = self.alphabet
        initial = frozenset(self.accepting)

        def follow(current, symbol):
            next_states = frozenset([
                prev
                for prev in self.transition
                for state in current
                if symbol in self.transition[prev] and
                self.transition[prev][symbol] == state
            ])
            if len(next_states) == 0:
                raise OblivionError
            return next_states

        def accepts(state):
            return self.initial in state

        return crawl(alphabet, initial, accepts, follow)

    def __reversed__(self):
        return self.reversed()

    def islive(self, state):
        """
        A state is alive if an accepting state can be reached from it.
        """
        seen = set([state])
        reachable = [state]
        i = 0
        while i < len(reachable):
            current = reachable[i]
            if current in self.accepting:
                return True
            if current in self.transition:
                for symbol in self.transition[current]:
                    next_state = self.transition[current][symbol]
                    if next_state not in seen:
                        reachable.append(next_state)
                        seen.add(next_state)
            i += 1
        return False

    def empty(self):
        """
        An FSM is empty if an accepting state can never be reached.
        """
        return not self.islive(self.initial)

    def __iter__(self):
        return self.strings()

    def equivalent(self, other):
        """
        Two regexes are the same if their symmetric difference is empty.
        """
        return (self ^ other).empty()

    def __eq__(self, other):
        return self.equivalent(other)

    def different(self, other):
        """
        FSMs are different if their symmetric difference is non empty.
        """
        return not (self ^ other).empty()

    def __ne__(self, other):
        return self.different(other)

    def difference(*fsms):
        return parallel(fsms,
                        lambda accepts: accepts[0] and not any(accepts[1:]))

    def __sub__(self, other):
        return self.difference(other)

    def isdisjoint(self, other):
        return (self & other).empty()

    def issubset(self, other):
        return (self - other).empty()

    def __le__(self, other):
        return self.issubset(other)

    def ispropersubset(self, other):
        return self <= other and self != other

    def __lt__(self, other):
        return self.ispropersubset(other)

    def issuperset(self, other):
        return (other - self).empty()

    def __ge__(self, other):
        return self.issuperset(other)

    def ispropersuperset(self, other):
        return self >= other and self != other

    def __gt__(self, other):
        return self.ispropersuperset(other)

    def derive(self, string):
        try:
            state = self.initial
            for symbol in string:
                if symbol not in self.alphabet:
                    if unspecified not in self.alphabet:
                        raise KeyError(symbol)
                    symbol = unspecified

                if not self.valid_transition(state, symbol):
                    raise OblivionError

                state = self.transition[state][symbol]

            return FSM(
                alphabet=self.alphabet,
                states=self.states,
                initial=state,
                accepting=self.accepting,
                transition=self.transition,
                __validation__=False
            )
        except OblivionError:
            return null(self.alphabet)


def null(alphabet):
    """
    An FSM accepting nothing.
    """
    return FSM(
        alphabet=alphabet,
        states={0},
        initial=0,
        accepting=set(),
        transition={
            0: dict([(symbol, 0) for symbol in alphabet]),
        },
        __validation__=False
    )


def epsilon(alphabet):
    """
    Create an FSM matching only the empty string.
    """
    return FSM(
        alphabet=alphabet,
        states={0},
        initial=0,
        accepting={0},
        transition={},
        __validation__=False
    )


def parallel(fsms, test):
    """
    Crawl several FSMs in parallel to create new FSM.
    """
    alphabet = set().union(*[fsm.alphabet for fsm in fsms])
    initial = {i: fsm.initial for (i, fsm) in enumerate(fsms)}

    def follow(current, symbol, fsm_range=tuple(enumerate(fsms))):
        next_states = dict()
        for i, f in fsm_range:
            if symbol not in f.alphabet and unspecified in f.alphabet:
                actual_symbol = unspecified
            else:
                actual_symbol = symbol
            if (i in current and current[i] in f.transition and
                    actual_symbol in f.transition[current[i]]):
                next_states[i] = f.transition[current[i]][actual_symbol]
        if not next_states:
            raise OblivionError
        return next_states

    def accepts(state, fsm_range=tuple(enumerate(fsms))):
        accepts = [i in state and state[i] in fsm.accepting
                   for (i, fsm) in fsm_range]
        return test(accepts)

    return crawl(alphabet, initial, accepts, follow)


def crawl(alphabet, initial, accepts, follow):
    """
    Create a new FSM from the above conditions.
    """

    states = [initial]
    accepting = set()
    transition = dict()

    i = 0
    while i < len(states):
        state = states[i]

        if accepts(state):
            accepting.add(i)

        transition[i] = dict()
        for symbol in alphabet:
            try:
                next_states = follow(state, symbol)
            except OblivionError:
                continue
            else:
                try:
                    j = states.index(next_states)
                except ValueError:
                    j = len(states)
                    states.append(next_states)
                transition[i][symbol] = j

        i += 1

    return FSM(
        alphabet=alphabet,
        states=range(len(states)),
        initial=0,
        accepting=accepting,
        transition=transition,
        __validation__=False
    )
