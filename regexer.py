import re

#TODO apple quotes

text = 'hello/reg/::/hi'

valid_pattern = re.compile(r'^(.+?)::(.+)$')
regex_pattern = re.compile(r'^(.*?)/(.+)/(.*?)$')
punctuation = re.compile(r'[!@#$%^&*(){}[]/?=+\\:;\'"-_`~]')

match = valid_pattern.match(text)

if text is None:
    exit("invalid input")
trigger = match.group(1)
response = match.group(2)

print(f"trigger: {trigger}")
print(f"response: {response}")

regex = regex_pattern.match(trigger)

if regex is None:
    print("trigger doesn't contain regex")
else:
    before = regex.group(1)
    after = regex.group(3)
    regex = regex.group(2)
    print(f"before: {before}")
    print(f"regex: {regex}")
    print(f"after: {after}")
