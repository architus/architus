from PIL import Image, ImageDraw
import io

sickle = Image.open('res/generate/gulag.png')


def generate(avatar: bytes) -> bytes:
    stream = io.BytesIO(avatar)
    img = Image.open(stream).resize(sickle.size)
    mask = Image.new("L", img.size, 0)
    black = Image.new("RGBA", img.size, 0)
    draw = ImageDraw.Draw(mask)
    draw.ellipse((0, 0) + img.size, fill=255)
    img = Image.composite(img, black, mask)
    img = Image.alpha_composite(img, sickle)
    buf = io.BytesIO()
    img.save(buf, 'PNG')
    buf.seek(0)
    return buf
