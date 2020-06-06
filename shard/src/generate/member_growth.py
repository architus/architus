import matplotlib
import io
from operator import attrgetter
matplotlib.use('agg')

import matplotlib.pyplot as plt


def generate(members):
    sums = []
    dates = []
    for i, m in enumerate(sorted(members, key=attrgetter('joined_at'))):
        if m.joined_at:
            sums.append(1 + sums[i - 1] if i > 0 else 0)
            dates.append(m.joined_at)

    fig, ax = plt.subplots()

    ax.plot(dates, sums)

    ax.set(xlabel='Date', ylabel='Members')
    ax.grid()
    plt.xticks(rotation=45)
    ax.legend()

    buf = io.BytesIO()
    plt.savefig(buf, bbox_inches='tight', edgecolor=None)

    plt.close()
    buf.seek(0)
    return buf.read()