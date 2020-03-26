import numpy as np
import operator
import random
import matplotlib
import io
matplotlib.use('agg')

import matplotlib.pyplot as plt
from matplotlib.ticker import MaxNLocator
from collections import namedtuple


COLORS = ['b','g','r','c','m','k']

def generate(message_counts, word_counts, victim) -> bytes:
    colors = random.sample(COLORS, 2)
    top_5_mesages = sorted(message_counts.items(), key=operator.itemgetter(1))[-5:]

    if victim and victim not in [m[0] for m in top_5_mesages]:
        top_5_mesages[0] = (victim, message_counts[victim])

    n_groups = len(top_5_mesages)


    fig, ax = plt.subplots()

    index = np.arange(n_groups)
    bar_width = 0.35

    opacity = 0.4
    error_config = {'ecolor': '0.3'}

    ax.bar(index, [count for _, count in reversed(top_5_mesages)], bar_width,
                    alpha=opacity, color=colors[0],
                    label='Messages')

    ax.bar(index + bar_width, [word_counts[member] for member, _ in reversed(top_5_mesages)], bar_width,
                    alpha=opacity, color=colors[1],
                    label='Words')

    ax.set_xlabel('User')
    ax.set_ylabel('Count')
    ax.set_xticks(index + bar_width / 2)
    ax.set_xticklabels([member.display_name for member, _ in reversed(top_5_mesages)])
    ax.legend()

    fig.tight_layout()

    buf = io.BytesIO()
    plt.savefig(buf, bbox_inches='tight', edgecolor=None)

    plt.close()
    buf.seek(0)
    return buf.read()
