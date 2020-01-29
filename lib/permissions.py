DEFAULT_PERMISSIONS = 0x112


class Permissions:
    flags = {
        'ADMINISTRATOR': 0x0001,
        'VIEW_PUBLIC_LOGS': 0x0002,
        'VIEW_PRIVATE_LOGS': 0x0004,
        'REVERT_LOG_ACTIONS': 0x0008,
        'ADD_AUTO_RESPONSE': 0x0010,
        'IGNORE_AUTO_RESPONSE_QUOTA': 0x0020,
        'EDIT_ANY_AUTO_RESPONSE': 0x0040,
        'DELETE_ANY_AUTO_RESPONSE': 0x0080,
        'VIEW_SETTINGS': 0x0100,
        'MANAGE_SETTINGS': 0x0200,
        'EXEC_PURGE_CMD': 0x0400,
    }

    def __init__(self, code: int = DEFAULT_PERMISSIONS):
        self.__dict__['code'] = code

    def __getattr__(self, name: str):
        try:
            return bool(self.code & Permissions.flags[name.upper()])
        except KeyError:
            raise AttributeError("Unknown Permission") from None

    def __setattr__(self, name: str, value: bool):
        try:
            flag = Permissions.flags[name.upper()]
        except KeyError:
            raise AttributeError("Unknown Permission") from None

        self.__dict__['code'] ^= flag if getattr(self, name) != value else 0

    def __and__(self, other):
        return Permissions(self.code & other)

    def __rand__(self, other):
        return other & self.code

    def __or__(self, other):
        return Permissions(self.code | other)

    def __ror__(self, other):
        return other | self.code

    def __xor__(self, other):
        return Permissions(self.code ^ other)

    def __rxor__(self, other):
        return other ^ self.code

    def __lshift__(self, other):
        return Permissions(self.code << other)

    def __rlshift__(self, other):
        return other << self.code

    def __rshift__(self, other):
        return Permissions(self.code >> other)

    def __rrshift__(self, other):
        return other >> self.code

    def __int__(self):
        return self.code

    def __str__(self):
        return hex(self.code)

    def __repr__(self):
        return f"Permissions: {hex(self.code)}"
