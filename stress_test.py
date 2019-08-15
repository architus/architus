from threading import Thread
from collections import Counter
import requests
from datetime import datetime
import time

results = []

def get(url):
    global results
    now = datetime.now()
    r = requests.get(url)
    results.append((r, (datetime.now() - now).total_seconds(), now))

if __name__ == '__main__':
    num = 250
    rate_ps = 10
    #url = 'https://api.archit.us:8000/stats/436189230390050826/messagecount'
    url = 'https://api.archit.us:8000/guild_count'
    print(f'{url} at {rate_ps} r/s')

    now = datetime.now()
    threads = [Thread(target=get, args=(url,)) for _ in range(num)]
    for thread in threads:
        thread.start()
        time.sleep(1/rate_ps)

    for thread in threads:
        thread.join()

    total_time = 0
    codes = []
    for result in sorted(results, key=lambda x: x[2]):
        print(f"{result[0].status_code} {result[1]:.2f} {'*' * round(result[1] * 10)}")
        total_time += result[1]
        codes.append(result[0].status_code)
    print('----------------')

    for code, count in Counter(codes).items():
        print(f"{code}s:       {count}")
    print(f"Avg time:   {total_time/num:.2f}s")
    print(f"Total time: {(datetime.now() - now).total_seconds()}s")
