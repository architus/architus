from wand.image import Image
from wand.drawing import Drawing
def generate(name, exclude):
    with Image(filename='res/generate/letmein.png') as img:
        with Drawing() as draw:
            count = 0
            new_exclude = ''
            scaler = .35
            size = 40
            for char in exclude:
                count += 1
                new_exclude += char
                if count % 15 < 7 and count > 7 and char == ' ':
                    new_exclude += '\n'
                    scaler -= .025
                    size -= 2
                    count = 0
            exclude = new_exclude

            draw.font = 'res/fonts/writing.ttf'
            draw.font_size = size
            draw.text(int(img.width / 1.7), int(img.height / 1.50), name)
            draw.text(int(img.width * .6), int(img.height * scaler), exclude)
            draw(img)

        img.format = 'png'
        img.save(filename='res/meme.png')
