#import youtube_dl
import youtube_dlc as youtube_dl
import functools
import pprint
from datetime import datetime
pp = pprint.PrettyPrinter(indent=1)
opts = {
    'prefer_ffmpeg': True,
    'format': 'bestaudio/best',
    'outtmpl': 'songs/%(extractor)s-%(id)s-%(title)s.%(ext)s',
    'restrictfilenames': True,
    #'noplaylist': True,
    'nocheckcertificate': True,
    'ignoreerrors': False,
    'logtostderr': False,
    #'quiet': True,
    #'no_warnings': True,
    'default_search': 'auto',
    'max_filesize': 52428800,
    'source_address': '0.0.0.0'  # bind to ipv4 since ipv6 addresses cause issues sometimes
}


def get_youtube_url(search, download=False, retries=0):
    '''scrape video url from search paramaters'''
    ydl = youtube_dl.YoutubeDL(opts)
    f = functools.partial(ydl.extract_info, search, download=download)
    try:
        data = f()
    except (youtube_dl.utils.ExtractorError, youtube_dl.utils.DownloadError) as e:
        print(e)
        if retries > 2:
            return
        return get_youtube_url(search, retries=retries + 1)
    try:
        pp.pprint(data['entries'][0])
        return [e['webpage_url'] for e in data['entries']]
    except KeyError as e:
        print(e)
        return []


searches = ['https://www.youtube.com/watch?v=n8X9_MgEdCg&list=PLuHCSw5Bii2DfT-nfIW2SkhI4QJC3ZwRD', 'monody', 'asnotehuanohu', 'imagine dragons - topic', "K/DA - MORE ft. Madison Beer, (G)I-DLE, Lexie Liu, Jaira Burns, Seraphine (Official Music Video)", "https://www.youtube.com/watch?v=AgpWX18dby4"]
now = datetime.now()
urls = [get_youtube_url(s, True) for s in searches]
print((datetime.now() - now).total_seconds())
print(urls)
now = datetime.now()
urls = [get_youtube_url(s) for s in searches]
print((datetime.now() - now).total_seconds())
print(urls)


#print([get_download_url(url) for url in urls if url is not None])
