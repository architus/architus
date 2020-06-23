from PIL import Image, ImageDraw, ImageFont
from io import BytesIO

font = ImageFont.truetype("res/fonts/uni-sans-semi-bold-webfont.ttf", 32)
font_color = (218, 219, 220, 255)
color = (47, 49, 54, 255)
emojis_per_page = 12

SIZE = 64

def generate(emojis):
    """
    Transform a list of emojis into a list of images.
    """

    """
    X_SIZE = max(1, len(emojis) // 15) * 480
    Y_SIZE = int(max(15, len(emojis)) * SIZE * 1.3 + 16)
    im = Image.new("RGBA", (X_SIZE, Y_SIZE), color)
    """
    """
    if len(emojis) > 25:
        im = Image.new('RGBA', (1200, 2080 + SIZE // 4), color)
    else:
    """
    images = []
    for v in range(0, len(emojis), emojis_per_page):
        im = Image.new('RGBA', (480, int(len(emojis[v:v + emojis_per_page]) * SIZE * 1.3) + SIZE // 4), color)

        for (i, e) in enumerate(emojis[v:v + emojis_per_page]):
            x = 16
            y = int(i * SIZE * 1.3) + 16
            im.alpha_composite(e.im.resize((SIZE, SIZE)), (x, y))
            d = ImageDraw.Draw(im)

            d.text((x + 80, y + 8), f":{e.name}:", fill=font_color, font=font)

        buf = BytesIO()
        im.save(buf, format="PNG")
        buf.seek(0)
        images.append(buf)
    return images

    #im.show()
