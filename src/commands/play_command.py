from src.commands.abstract_command import abstract_command
import re
import discord

class play_command(abstract_command):

    def __init__(self):
        super().__init__("play")

    async def exec_cmd(self, **kwargs):
        players = kwargs['players']
        player = players[self.server.id]
        await self.client.send_typing(self.channel)
        if not discord.opus.is_loaded():
            discord.opus.load_opus('res/libopus.so')
        if not (player.is_connected()):
            voice = await self.client.join_voice_channel(self.author.voice.voice_channel)
            player.voice = voice
        else:
            player.voice.move_to(self.author.voice.voice_channel)

        arg = self.content.split(' ')
        add = arg[0] == '!add'
        message = ''
        if (len(arg) > 1):
            if ('/playlist/' in arg[1]):
                try:
                    urls = await player.add_spotify_playlist(arg[1])
                    message = "Queuing \"" + urls[0] + "\"."
                    del urls[0]
                    await player.add_url(urls[0])
                    name = await player.play()
                    for track in urls:
                        await player.add_url(track)
                    if (name):
                        message += "\nPlaying: " + name
                except:
                    message = "something went badly wrong please spam my creator with pings"
            elif ('/track/' in arg[1]):
                if (add):
                    name = await player.add_url(arg[1]);
                    if (name):
                        message = 'Added: ' + name
                else:
                    await player.add_url_now(arg[1]);
                    name = await player.play()
                    if (name):
                        message = "Now playing: " + name
            elif ('youtu' in arg[1]):
                if (add):
                    await player.add_url(arg[1])
                    message = 'Added'
                else:
                    player.pause()
                    await player.add_url_now(arg[1])
                    name = await player.play()
                    if (name):
                        message = "Playing " + name
            elif ('town' in arg[1] or 'encounter' in arg[1] or 'boss' in arg[1] or 'exploration' in arg[1]):
                message = "Please pass in the url of the playlist."
            else:
                del arg[0]
                url = await player.get_youtube_url(' '.join(arg))
                if (add):
                    await player.add_url(url)
                    message = "Added: " + url
                else:
                    await player.add_url_now(url)
                    name = await player.play()
                    if (name):
                        message = "Now Playing: " + url
        else:
            if (len(player.q) == 0):
                message = "Play what, " + self.author.mention + "?"
            else:
                name = await player.play()
                if (name):
                    message = "Now playing: " + name

        print ('q ' + str(len(player.q)))
        for song in list(player.q):
            print ('song: ' + song)

        await self.client.send_message(self.channel, message)

    def get_help(self):
        return "Add a song to the queue or play immediately. Supports youtube and spotify links."

    def get_usage(self):
        return "(<url> | <search>)"
