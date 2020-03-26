from PIL import Image, ImageDraw
import io

sickle = Image.open('res/generate/gulag.png')
black = Image.new("RGBA", sickle.size, 0)
mask = Image.new("L", sickle.size, 0)
draw = ImageDraw.Draw(mask)
draw.ellipse((0, 0) + sickle.size, fill=255)


def generate(avatar: bytes) -> bytes:
    stream = io.BytesIO(avatar)
    img = Image.open(stream).resize(sickle.size)
    img = Image.composite(img, black, mask)
    img = Image.alpha_composite(img, sickle)
    buf = io.BytesIO()
    img.save(buf, 'PNG')
    buf.seek(0)
    return buf
