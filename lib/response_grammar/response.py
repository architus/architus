from enum import Enum, auto
import re2 as re
import json

just_shortcode = re.compile(r"\[:([A-Za-z]+):\]")
shortcode_id = re.compile(r"\[<:([A-Za-z]+):(\\d+)>\]")
animated = re.compile(r"\[<a:([A-Za-z]+):(\\d+)>\]")
capture = re.compile("\[(\\d+)\]")
url = re.compile("(https?://[\\w\\.-]{2,})")


class ParseError(Exception):
    def __init__(self, message, position):
        self.message = message
        self.position = position


class NodeType(Enum):
    Root = auto(),
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
    Capture = auto(),
    Url = auto()


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

    def stringify(self):
        return tree_string(self)


def tree_string(node, tree=[]):
    if (node.type == NodeType.List):
        tree.append(("open", "["))
        for c in node.children:
            tree = tree_string(c, tree)
        tree.append(("close", "]"))
        return tree
    elif (node.type == NodeType.ListElement):
        for c in node.children:
            tree = tree_string(c, tree)
        return tree
    elif (node.type == NodeType.PlainText):
        tree.append(("text", node.text))
        return tree
    elif (node.type == NodeType.React):
        tree.append(("react", node.text))
        return tree
    elif (node.type == NodeType.Noun):
        tree.append(("noun", node.text))
        return tree
    elif (node.type == NodeType.Adj):
        tree.append(("adj", node.text))
        return tree
    elif (node.type == NodeType.Adv):
        tree.append(("adv", node.text))
        return tree
    elif (node.type == NodeType.Count):
        tree.append(("count", node.text))
        return tree
    elif (node.type == NodeType.Member):
        tree.append(("member", node.text))
        return tree
    elif (node.type == NodeType.Author):
        tree.append(("author", node.text))
        return tree
    elif (node.type == NodeType.Capture):
        tree.append(("capture", node.text))
        return tree
    elif (node.type == NodeType.Url):
        tree.append(("url", node.text))
        return tree
    else:
        tree = []
        for c in node.children:
            tree = tree_string(c, tree)
        return tree


def parse_react(string, i=0):
    end = string.find(']', i)
    m = just_shortcode.fullmatch(string[i:end+1])
    if m is not None:
        groups = m.groups()
        return (end + 1, False, groups[0], None)
    m = shortcode_id.fullmatch(string[i:end+1])
    if m is not None:
        groups = m.groups()
        return (end + 1, False, groups[0], int(groups[1]))
    m = animated.fullmatch(string[i:end+1])
    if m is not None:
        groups = m.groups()
        return (end + 1, False, groups[0], int(groups[1]))
    return (i, False, None, None)


def parse_capture(string, i=0):
    end = string.find(']', i)
    m = capture.fullmatch(string[i:end+1])
    if m is not None:
        groups = m.groups()
        return (end + 1, int(groups[0]))
    return i, None


def parse(string):
    base = Response()
    curr = base
    root_ctx = True
    i = 0
    while i < len(string):
        root_ctx = True if curr == base else False
        if string[i] == '[' and string.find(']', i) == -1:
            raise ParseError("Unmatched bracket", i)
        if string[i] == ']':
            curr = curr.parent
            if curr == None:
                raise ParseError("This bracket has no opening bracket", i + 1)
            curr = curr.parent
            if curr == None:
                raise ParseError("This bracket has no opening bracket", i + 1)
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
                node.capture_group = capture
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
        elif string[i] == "," and not root_ctx:
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
                if string[i] != " ":
                    ends = [string.find(" ", i), string.find("]", i), string.find(",", i)]
                    end = len(string)
                    for e in ends:
                        if e != -1 and e < end:
                            end = e
                    m = url.fullmatch(string[i:end])
                    if m is not None:
                        if text != "":
                            node = Node()
                            node.type = NodeType.PlainText
                            node.text = text
                            node.parent = curr
                            curr.children.append(node)
                        node = Node()
                        node.type = NodeType.Url
                        node.text = string[i:end]
                        node.parent = curr
                        curr.children.append(node)
                        text = ""
                        i = end
                        continue
                if string[i] == "," and curr.type == NodeType.ListElement:
                    break
                text += string[i]
                i += 1
            if text != "":
                node = Node()
                node.type = NodeType.PlainText
                node.text = text
                node.parent = curr
                curr.children.append(node)

    if curr != base:
        raise ParseError("Missed a closing bracket somewhere", len(string))
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
    print(tree.stringify())
