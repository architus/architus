#import youtube_dl
import youtube_dlc as youtube_dl
import functools
import pprint
pp = pprint.PrettyPrinter(indent=1)
opts = {
    'prefer_ffmpeg': True,
    'format': 'bestaudio/best',
    'outtmpl': '%(extractor)s-%(id)s-%(title)s.%(ext)s',
    'restrictfilenames': True,
    'noplaylist': True,
    'nocheckcertificate': True,
    'ignoreerrors': False,
    'logtostderr': False,
    'quiet': True,
    'no_warnings': True,
    'default_search': 'auto',
    'source_address': '0.0.0.0'  # bind to ipv4 since ipv6 addresses cause issues sometimes
}


def get_youtube_url(search, retries=0):
    '''scrape video url from search paramaters'''
    ydl = youtube_dl.YoutubeDL(opts)
    f = functools.partial(ydl.extract_info, search, download=False)
    try:
        data = f()
    except (youtube_dl.utils.ExtractorError, youtube_dl.utils.DownloadError) as e:
        print(e)
        if retries > 2:
            return
        return get_youtube_url(search, retries=retries + 1) 
    if data['_type'] == 'playlist':
        try:
            data = data['entries'][0]
        except IndexError:
            print("No video found matching that search")
            return
    try:
        return data['webpage_url']
    except KeyError as e:
        print(e)
        return 

def get_download_url(url):
    ffmpeg_options = {
        'options': '-vn -reconnect 1 -reconnect_streamed 1 -reconnect_delay_max 5'
    }
    ydl = youtube_dl.YoutubeDL(opts)
    func = functools.partial(ydl.extract_info, url, download=True)
    try:
        info = func()
    except youtube_dl.utils.ExtractorError as e:
        print(e)
        return None
    except youtube_dl.utils.DownloadError as e:
        print(e)
        return None
    #print(info)
    if "entries" in info:
        info = info['entries'][0]

    download_url = info['url']
    #download_url = ydl.prepare_filename(info)
    return download_url
#self.voice.play(discord.FFmpegPCMAudio(download_url, **ffmpeg_options), after=self.agane)
searches = ['masquerade', 'monody', 'asnotehuanohu', 'imagine dragons - topic', "K/DA - MORE ft. Madison Beer, (G)I-DLE, Lexie Liu, Jaira Burns, Seraphine (Official Music Video)"]
urls = [get_youtube_url(s) for s in searches]
print(urls)
urls = [get_youtube_url(s) for s in searches]
print(urls)
urls = [get_youtube_url(s) for s in searches]
print(urls)
urls = [get_youtube_url(s) for s in searches]
print(urls)
urls = [get_youtube_url(s) for s in searches]
print(urls)

#print([get_download_url(url) for url in urls if url is not None])
