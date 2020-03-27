from datetime import datetime, timezone
import matplotlib
import io
matplotlib.use('agg')

import matplotlib.pyplot as plt


def generate(data, deaths_only=False):
    '''takes data in a cryptic format to make a pretty graph'''
    times = []
    positives = []
    deaths = []

    for entry in data:
        times.append(datetime.fromtimestamp(int(entry[2]), timezone.utc))
        positives.append(int(entry[4]))
        deaths.append(int(entry[5]))

    fig, ax = plt.subplots()
    if not deaths_only:
        ax.plot(times, positives, label="Positive")
    ax.plot(times, deaths, label="Deaths")

    ax.set(xlabel='Date', ylabel='People', title='Coronavirus')
    ax.grid()
    ax.legend()

    buf = io.BytesIO()
    plt.savefig(buf, bbox_inches='tight', edgecolor=None)

    plt.close()
    buf.seek(0)
    return buf.read()
