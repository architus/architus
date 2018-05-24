import numpy as np
import random
import matplotlib
matplotlib.use('Agg')
import matplotlib.pyplot as plt

def generate(x, y, names):

    #plt.axis('off')
    fig, ax = plt.subplots()
    ax.tick_params(axis='both',
    which='both', bottom=False, left=False,
    top=False, labelbottom=False,
    labelleft=False, labelright=False)
    plt.text(-11,0, 'Toxic', horizontalalignment='right',
            verticalalignment='center',
            fontsize='20', color='blue',
            rotation='vertical')
    plt.text(11.5,0, 'Nice', horizontalalignment='right',
            verticalalignment='center',
            fontsize='20', color='blue',
            rotation='vertical')
    plt.text(0,11, 'Autistic', horizontalalignment='center',
            verticalalignment='center',
            fontsize='20', color='blue')
    plt.text(0,-12, 'Normie', horizontalalignment='center',
            verticalalignment='center',
            fontsize='20', color='blue')

    ax.plot(x, y, 'ro')
    plt.axis([-10, 10, -10, 10])
    ax.axhline(y=0, color='k')
    ax.axvline(x=0, color='k')
    for i in range(len(names)):
        offset = list(range(-20,-10)) + list(range(10,20))
        xoff = random.choice(offset) / 10
        yoff = random.choice(offset) / 10
        plt.annotate(names[i], xy=(x[i], y[i]), xytext=(x[i]+xoff, y[i]+yoff),
                fontsize=10,

                    arrowprops=dict(facecolor='black', shrink=0.05),)
    #plt.grid(True)
    plt.savefig('res/foo.png', bbox_inches='tight')
    plt.close()
#    plt.show()


