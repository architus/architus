# First import everthing you need
import numpy as np
import random
import itertools
import matplotlib
from matplotlib import pyplot as plt
from matplotlib import animation
from mpl_toolkits.mplot3d import Axes3D
import os

# Create some random data, I took this piece from here:
# http://matplotlib.org/mpl_examples/mplot3d/scatter3d_demo.py
def generate(xx, yy, zz, names, title, key):
    #xx = range(-10,15)
    #yy = range(-10,15)
    #zz = [1,1,1,1,1,1,1,1,1,1,1,1,1,-1,1,1,1,1,1,1,2,3,4,5,6]

# Create a figure and a 3D Axes
    plt.title = 'hello'
    count = 0
    fig = plt.figure()
    ax = Axes3D(fig)
#for i in range(len(xx)):
            #ax.text(xx[i], yy[i], zz[i], i)

    col = np.arange(1)

    def init():
        #count += 1
        ax.set_xlabel('$toxicity$', fontsize=10)
        ax.set_ylabel('$autism$', fontsize=10)
        ax.set_zlabel('$idiocy$', fontsize=10)
        ax.set_title(title)
        ax.tick_params(axis='both',
        which='minor', bottom=True, left=True, width=2,
        top=False, labelbottom=True,
        labelleft=False, labelright=False)
        marker = itertools.cycle(('+', 'o', '*', '8', 's', 'p', 'H', 'D', 'v', '^', '<', '>', '1', '2', '3', '4', 'h', 'd', 'P', 'x', 'X')) 
        if count > 1:
            raise Exception('hi')
        for i in range(len(xx)):
            ax.scatter(xx[i:i+1], yy[i:i+1], zz[i:i+1], marker=next(marker), s=20, label=names[i])
        if len(names) < 20:
            leg = ax.legend(loc=3)
        else:
            leg = ax.legend(loc=3, prop={'size': 6})
        leg.get_frame().set_alpha(0.7)
        #ax.scatter(xx, yy, zz, marker='o', c=col, s=20, alpha=0.6)

        return fig,

    def animate(i):
        ax.view_init(elev=10., azim=i)
        return fig,

# Animate
    anim = animation.FuncAnimation(fig, animate, init_func=init,
                                frames=360, interval=20, blit=True)
# Save
#anim.save('res/animation.gif', writer='imagemagick', fps=5)
    ax.clear()

    anim.save('res/%spart.webm' % key, fps=18, extra_args=['-vcodec', 'libvpx-vp9'])
    os.rename('res/%spart.webm' % key, 'res/%s.webm' % key)
#writer = FFMpegWriter(fps=15, codec='libvpx-vp9') # or libvpx-vp8

