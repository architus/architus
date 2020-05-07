from enum import Enum, auto
import re2 as re
import json

just_shortcode = re.compile(r"\[:([A-Za-z]+):\]")
shortcode_id = re.compile(r"\[<:([A-Za-z]+):(\\d+)>\]")
animated = re.compile(r"\[<a:([A-Za-z]+):(\\d+)>\]")
capture = re.compile("\[(\\d+)\]")


class ParseError(Exception):
    pass


class NodeType(Enum):
    Root, = auto(),
    List = auto(),
    ListElement = auto(),
    PlainText = auto(),
    React = auto(),
    Noun = auto(),
    Adj = auto(),
    Adv = auto(),
    Count = auto(),
    Member = auto(),
    Author = auto(),
    Capture = auto()


def serialize(obj):
    if isinstance(obj, NodeType):
        return {'NodeType': str(obj).split(".")[1]}
    if isinstance(obj, Node):
        return {
            'type': obj.type,
            'text': obj.text,
            'children': obj.children,
            'shortcode': obj.shortcode,
            'id': obj.id,
            'animated': obj.animated,
            'capture_group': obj.capture_group
        }
    if isinstance(obj, Response):
        return {
            'type': obj.type,
            'children': obj.children
        }
    return obj


class Node:
    def __init__(self):
        self.type = None
        self.text = None
        self.parent = None
        self.children = []
        self.shortcode = None
        self.id = -1
        self.animated = False
        self.capture_group = -1


class Response:
    def __init__(self):
        self.children = []
        self.parent = None
        self.type = NodeType.Root

    def to_json(self):
        return json.dumps(self, default=serialize)


def parse_react(string, i=0):
    end = string.find(']', i)
    if end == -1:
        raise ParseError
    m = just_shortcode.fullmatch(string[i:end+1])
    if m is not None:
        groups = m.groups()
        return (end + 1, False, groups[0], None)
    m = shortcode_id.fullmatch(string[i:end+1])
    if m is not None:
        groups = m.groups()
        return (end + 1, False, groups[0], int(groups[1]))
    m = shortcode_id.fullmatch(string[i:end+1])
    if m is not None:
        groups = m.groups()
        return (end + 1, False, groups[0], int(groups[1]))
    return (i, False, None, None)


def parse_capture(string, i=0):
    end = string.find(']', i)
    if end == -1:
        raise ParseError
    m = capture.fullmatch(string[i:end+1])
    if m is not None:
        groups = m.groups()
        return (end + 1, int(groups[0]))
    return i, None


def parse(string):
    base = Response()
    curr = base
    i = 0
    while i < len(string):
        if curr == None:
            raise ParseError
        if string[i] == ']':
            curr = curr.parent
            curr = curr.parent
            i += 1
            continue
        elif string[i] == '[':
            j, a, shortcode, cid = parse_react(string, i)
            k, capture = parse_capture(string, i)
            if shortcode is not None:
                node = Node()
                node.type = NodeType.React
                node.parent = curr
                node.shortcode = shortcode
                node.animated = a
                node.id = cid
                node.text = string[i:j]
                curr.children.append(node)
                i = j
            elif capture is not None:
                node = Node()
                node.type = NodeType.Capture
                node.parent = curr
                node.capture = capture
                node.text = string[i:k]
                curr.children.append(node)
                i = k
            elif i + 5 <= len(string) and string[i:i+5].lower() == '[adv]':
                node = Node()
                node.type = NodeType.Adv
                node.text = string[i:i+5]
                node.parent = curr
                curr.children.append(node)
                i += 5
            elif i + 5 <= len(string) and string[i:i+5].lower() == '[adj]':
                node = Node()
                node.type = NodeType.Adj
                node.text = string[i:i+5]
                node.parent = curr
                curr.children.append(node)
                i += 5
            elif i + 6 <= len(string) and string[i:i+6].lower() == '[noun]':
                node = Node()
                node.type = NodeType.Noun
                node.text = string[i:i+6]
                node.parent = curr
                curr.children.append(node)
                i += 6
            elif i + 7 <= len(string) and string[i:i+7].lower() == '[count]':
                node = Node()
                node.type = NodeType.Count
                node.text = string[i:i+7]
                node.parent = curr
                curr.children.append(node)
                i += 7
            elif i + 8 <= len(string) and string[i:i+8].lower() == '[member]':
                node = Node()
                node.type = NodeType.Member
                node.text = string[i:i+8]
                node.parent = curr
                curr.children.append(node)
                i += 8
            elif i + 8 <= len(string) and string[i:i+8].lower() == '[author]':
                node = Node()
                node.type = NodeType.Author
                node.text = string[i:i+8]
                node.parent = curr
                curr.children.append(node)
                i += 8
            elif i + 9 <= len(string) and string[i:i+9].lower() == '[capture]':
                node = Node()
                node.type = NodeType.Capture
                node.text = string[i:i+9]
                node.parent = curr
                curr.children.append(node)
                i += 9
            else:
                node = Node()
                node.type = NodeType.List
                node.parent = curr
                curr.children.append(node)
                curr = node
                node = Node()
                node.type = NodeType.ListElement
                node.parent = curr
                curr.children.append(node)
                curr = node
                i += 1
            continue
        elif string[i] == ",":
            curr = curr.parent
            node = Node()
            node.parent = curr
            node.type = NodeType.ListElement
            curr.children.append(node)
            curr = node
            i += 1
        else:
            text = ""
            while i < len(string) and string[i] != "[" and string[i] != "]":
                if string[i] == "\\":
                    text += string[i+1]
                    i += 2
                    continue
                if string[i] == "," and curr.type == NodeType.ListElement:
                    break
                text += string[i]
                i += 1
            if text.strip() != "":
                node = Node()
                node.type = NodeType.PlainText
                node.text = text.strip()
                node.parent = curr
                curr.children.append(node)

    if curr != base:
        raise ParseError
    return base


def walk(tree, discovered=[]):
    if hasattr(tree, "text"):
        if tree.text != None:
            print(f"( {tree.type}: {tree.text}")
        else:
            print(f"( {tree.type}")
    else:
        print(f"( {tree.type}")
    for c in tree.children:
        if c not in discovered:
            discovered.append(c)
            discovered = walk(c, discovered)
    print(")")
    return discovered


if __name__ == "__main__":
    with open("example.txt", "r") as f:
        data = f.read()
    data = data.strip()

    tree = parse(data)
    print(tree.to_json())
