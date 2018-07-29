import numpy as np
import itertools
import random
import matplotlib
matplotlib.use('Agg')

import matplotlib.pyplot as plt

def generate(x, y, names):

    #plt.axis('off')
    font = matplotlib.font_manager.FontProperties(fname='res/fonts/NotoSansSymbols-Regular.ttf',
                                   style='normal', size=16)

    marker = itertools.cycle(('+', 'o', '*', '8', 's', 'p', 'H', 'D', 'v', '^', '<', '>', '1', '2', '3', '4', 'h', 'd', 'P', 'x', 'X')) 
    fig, ax = plt.subplots()
    #ax.set_facecolor('#23272A')
    #ax.set_facecolor('white')
    fig.patch.set_alpha(0.7)
    ax.patch.set_alpha(0.3)
    ax.set_alpha(0.5)
    ax.tick_params(axis='both',
    which='both', bottom=False, left=False,
    top=False, labelbottom=False,
    labelleft=False, labelright=False)
    plt.text(-10.5,0, 'Toxic', horizontalalignment='right',
            verticalalignment='center',
            fontsize='12', color='blue',
            rotation='vertical')
    plt.text(10.9,0, 'Nice', horizontalalignment='right',
            verticalalignment='center',
            fontsize='12', color='blue',
            rotation='vertical')
    plt.text(0,11, 'Autistic', horizontalalignment='center',
            verticalalignment='center',
            fontsize='12', color='blue')
    plt.text(0,-11, 'Normie', horizontalalignment='center',
            verticalalignment='center',
            fontsize='12', color='blue')

    plt.axis([-10, 10, -10, 10])
    ax.axhline(y=0, color='k', linewidth=1)
    ax.axvline(x=0, color='k', linewidth=1)
    for i in range(len(names)):
        ax.plot(x[i], y[i], marker=next(marker), label=names[i])
        offset = list(range(-20,-10)) + list(range(10,20))
        xoff = random.choice(offset) / 10
        yoff = random.choice(offset) / 10
        #plt.annotate(names[i], xy=(x[i], y[i]), xytext=(x[i]+xoff, y[i]+yoff),
                #fontsize=10, arrowprops=dict(facecolor='black', shrink=0.05),)
        leg = ax.legend(bbox_to_anchor=(1.05, 1), loc=2, borderaxespad=0., prop=font)
        leg.get_frame().set_alpha(0.5)
    #plt.grid(True)
    plt.savefig('res/foo.png', bbox_inches='tight', edgecolor=None)
    plt.close()
#    plt.show()


