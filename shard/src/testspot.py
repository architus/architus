import spotify_tools

import pprint
from datetime import datetime
pp = pprint.PrettyPrinter(indent=1)

pp.pprint(spotify_tools.fetch_playlist("https://open.spotify.com/playlist/2LKoqgZlTfVvr36HksruHi?si=c6_dyrWRRpanYEZ-hUdldA"))