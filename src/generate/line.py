import random
import string
from wand.image import Image
from wand.drawing import Drawing
def generate(line, normal, yikes):
    with Image(filename='res/generate/line.png') as img:
        with Drawing() as draw:
            count = 0
            new_line = ''
            scaler = .35
            size = 40
            for char in line:
                count += 1
                new_line += char
                if count % 15 < 7 and count > 7 and char == ' ':
                    new_exclude += '\n'
                    scaler -= .025
                    size -= 2
                    count = 0
            line = new_line

            draw.font = 'res/fonts/writing.ttf'
            draw.font_size = size
            draw.text(int(img.width * .2), int(img.height * .7), line)
            draw.text(int(img.width * .2), int(img.height * .42), yikes)
            draw.text(int(img.width * .2), int(img.height * .16), normal)
            draw(img)

        img.format = 'png'
        name = ''.join([random.choice(string.ascii_letters) for n in range(10)])
        img.save(filename='res/line.png')
        return name

if __name__ == '__main__':
    print(generate('line a long line', 'normal', 'yikes'))
