from PIL import Image, ImageDraw, ImageFont
from io import BytesIO

font = ImageFont.truetype("res/fonts/uni-sans-semi-bold-webfont.ttf", 32)
font_color = (218, 219, 220)
color = (47,49,54)

SIZE = 64

def generate(emojis):
    '''takes a list of architus emojis and makes an image'''

    im = Image.new('RGB', (600, int(len(emojis) * SIZE * 1.3) + SIZE // 4), color)

    for (i, e) in enumerate(emojis):
        im.paste(e.im.resize((SIZE, SIZE)), (16, int(i * SIZE * 1.3) + 16))
        d = ImageDraw.Draw(im)

        d.text((96, int(i * SIZE * 1.3) + 16 + 8), f":{e.name}:", fill=font_color, font=font)

    buf = BytesIO()
    im.save(buf, format="PNG")
    buf.seek(0)
    return buf

    #im.show()
