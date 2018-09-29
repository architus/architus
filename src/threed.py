# First import everthing you need
import numpy as np
import random
import itertools
import matplotlib
from matplotlib import pyplot as plt
from matplotlib import animation
from mpl_toolkits.mplot3d import Axes3D

# Create some random data, I took this piece from here:
# http://matplotlib.org/mpl_examples/mplot3d/scatter3d_demo.py
def generate(xx, yy, zz, names):
    #xx = range(-10,15)
    #yy = range(-10,15)
    #zz = [1,1,1,1,1,1,1,1,1,1,1,1,1,-1,1,1,1,1,1,1,2,3,4,5,6]

# Create a figure and a 3D Axes
    plt.title = 'hello'
    count = 0
    fig = plt.figure()
    ax = Axes3D(fig)
    ax.set_xlabel('$toxicity$', fontsize=10)
    ax.set_ylabel('$autism$', fontsize=10)
    ax.set_zlabel('$idiocy$', fontsize=10)
    ax.set_title('spectrum')
    ax.tick_params(axis='both',
    which='minor', bottom=True, left=True, width=2,
    top=False, labelbottom=True,
    labelleft=False, labelright=False)
    leg = ax.legend(xx, bbox_to_anchor=(1.05,1), loc=4)
    leg.get_frame().set_alpha(0.5)
#for i in range(len(xx)):
            #ax.text(xx[i], yy[i], zz[i], i)

# Create an init function and the animate functions.
# Both are explained in the tutorial. Since we are changing
# the the elevation and azimuth and no objects are really
# changed on the plot we don't have to return anything from
# the init and animate function. (return value is explained
# in the tutorial.
#%% Create Color Map
#colormap = plt.get_cmap("YlOrRd")
#norm = matplotlib.colors.Normalize(vmin=-10, vmax=10)
    col = np.arange(1)

    def init():
        #count += 1
        marker = itertools.cycle(('+', 'o', '*', '8', 's', 'p', 'H', 'D', 'v', '^', '<', '>', '1', '2', '3', '4', 'h', 'd', 'P', 'x', 'X')) 
        if count > 1:
            raise Exception('hi')
        for i in range(len(xx)):
            ax.scatter(xx[i:i+1], yy[i:i+1], zz[i:i+1], marker=next(marker), s=20, label=names[i])
        leg = ax.legend(loc=3)
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

    anim.save('res/foo.webm', fps=18, extra_args=['-vcodec', 'libvpx-vp9'])
#writer = FFMpegWriter(fps=15, codec='libvpx-vp9') # or libvpx-vp8

