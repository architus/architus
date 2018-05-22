secret_token = None

try:
    lines = [line.rstrip('\n') for line in open('.secret_token')]
    secret_token = lines[0]

except Exception as e:
    print('error reading .secret_token, make it you aut')
