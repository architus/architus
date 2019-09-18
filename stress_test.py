from multiprocessing.pool import ThreadPool
from collections import Counter
import requests
from datetime import datetime


def get(url):
    now = datetime.now()
    r = requests.get(url)
    return r, (datetime.now() - now).total_seconds()


if __name__ == '__main__':
    num = 400
    url = 'https://api.archit.us:8000/guild_count'

    now = datetime.now()
    with ThreadPool(num) as p:
        results = p.map(get, [url for _ in range(num)])

    total_time = 0
    codes = []
    for result in results:
        total_time += result[1]
        codes.append(result[0].status_code)

    print(url)
    for code, count in Counter(codes).items():
        print(f"{code}s        {count}")
    print(f"Avg time:   {total_time/num:.2f}s")
    print(f"Total time: {(datetime.now() - now).total_seconds()}s")
