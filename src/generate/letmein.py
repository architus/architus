from wand.image import Image
from wand.drawing import Drawing
def generate(name, exclude):
    with Image(filename='res/generate/letmein.png') as img:
        with Drawing() as draw:

            draw.font = 'res/fonts/writing.ttf'
            draw.font_size = 40
            draw.text(int(img.width / 1.7), int(img.height / 1.50), name)
            draw.text(int(img.width / 1.7), int(img.height / 2.83), exclude)
            draw(img)

        img.format = 'png'
        img.save(filename='res/meme.png')
