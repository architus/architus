import spotipy
import spotipy.oauth2 as oauth2
import lyricwikia

from titlecase import titlecase


def generate_token():
    """ Generate the token. Please respect these credentials :) """
    credentials = oauth2.SpotifyClientCredentials(
        client_id='1b4fadd2be3b48bda672c6e67caf7957',
        client_secret='4c1fba736efb4dd8a363c4bdbac6ce81')
    token = credentials.get_access_token()
    return token


# token is mandatory when using Spotify's API
# https://developer.spotify.com/news-stories/2017/01/27/removing-unauthenticated-calls-to-the-web-api/
token = generate_token()
spotify = spotipy.Spotify(auth=token)


def generate_metadata(raw_song):
    """ Fetch a song's metadata from Spotify. """
    meta_tags = spotify.track(raw_song)
    artist = spotify.artist(meta_tags['artists'][0]['id'])
    album = spotify.album(meta_tags['album']['id'])

    """     try:
        meta_tags[u'genre'] = titlecase(artist['genres'][0])
    except IndexError:
        meta_tags[u'genre'] = None
    try:
        meta_tags[u'copyright'] = album['copyrights'][0]['text']
    except IndexError:
        meta_tags[u'copyright'] = None
    try:
        meta_tags[u'external_ids'][u'isrc']
    except KeyError:
        meta_tags[u'external_ids'][u'isrc'] = None

    meta_tags[u'release_date'] = album['release_date']
    meta_tags[u'publisher'] = album['label']
    meta_tags[u'total_tracks'] = album['tracks']['total']

    try:
        meta_tags['lyrics'] = lyricwikia.get_lyrics(meta_tags['artists'][0]['name'], meta_tags['name'])
    except lyricwikia.LyricsNotFound:
        meta_tags['lyrics'] = None

    # Some sugar
    meta_tags['year'], *_ = meta_tags['release_date'].split('-')
    meta_tags['duration'] = meta_tags['duration_ms'] / 1000.0
    # Remove unwanted parameters
    del meta_tags['duration_ms']
    del meta_tags['available_markets']
    del meta_tags['album']['available_markets'] """

    return meta_tags


def fetch_playlist(playlist):
    splits = get_splits(playlist)
    try:
        username = splits[-3]
    except IndexError:
        # Wrong format, in either case
        raise
    playlist_id = splits[-1]
    try:
        results = spotify.user_playlist(username, playlist_id,
                                        fields='tracks,next,name')
    except spotipy.client.SpotifyException:
        raise

    return results


def fetch_spotify_playlist_songs(url):
    urls = []
    data = fetch_playlist(url)
    # print(data)
    # urls.append(data['name'])
    tracks = data['tracks']
    for item in tracks['items']:
        if 'track' in item:
            track = item['track']
        else:
            track = item
        try:
            track_url = track['external_urls']['spotify']
            urls.append(track_url)
        except KeyError:
            pass

    return data['name'], urls


def spotify_to_str(url: str) -> str:
    data = generate_metadata(url)
    return "%s - %s" % (data['name'], data['artists'][0]['name'])


def get_splits(url):
    if '/' in url:
        if url.endswith('/'):
            url = url[:-1]
        splits = url.split('/')
    else:
        splits = url.split(':')
    return splits
