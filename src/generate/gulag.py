from wand.image import Image
import urllib.request as urllib2
from wand.drawing import Drawing
from wand.drawing import Color
import random, os, string

from PIL import Image as pilimg

def generate(url):
    hdr = {
            'User-Agent':'Mozilla/5.0',
            'Accept': 'text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8'
        }
    req = urllib2.Request(url, headers=hdr)
    f = urllib2.urlopen(req)

    im = pilimg.open(f)
    key = ''.join([random.choice(string.ascii_letters) for n in range(10)])
    im.save('res/' + key + ".png","png")

    with Image(filename='res/%s.png' % key) as img:
        if img.sequence:
            img = Image(image=img.sequence[0])
        img.resize(300,300)
        with Drawing() as draw:
            draw.fill_color = Color('#f00')
            draw.fill_opacity = 0.4
            draw.rectangle(0, 0, img.width, img.height)

            draw(img)
        with Image(filename='res/generate/gulag.png') as sickle:
            sickle.resize(200,200)
            img.composite(sickle, 50, 50)
        img.format = 'png'
        img.save(filename='res/gulag.png')
    os.remove('res/' + key + '.png')


#generate('https://cdn.discordapp.com/avatars/131294120399470602/a_789c7272a7cf92f7450df65adc4c856c.gif?size=1024')
generate('https://cdn.discordapp.com/avatars/214037134477230080/f8bbce770f9422229b19425c9e4191fe.webp?size=1024')
