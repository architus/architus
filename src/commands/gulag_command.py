from src.commands.abstract_command import abstract_command
from discord import ChannelType
import src.generate.gulag as gulaggen
import time
import discord

GULAG_THRESHOLD = 5
GULAG_TIME = 5
GULAG_TIME_ADD = 2
class gulag_command(abstract_command):

    def __init__(self):
        super().__init__("gulag")

    async def exec_cmd(self, **kwargs):
        server = self.server
        settings = kwargs['settings']
        filtered = filter(lambda role: role.name == "kulak", server.role_hierarchy)
        try:
            gulag_role = next(filtered)
            gulag_emoji = self.get_custom_emoji(server, "gulag")
        except:
            print("gulag role/emoji not found")
            await self.channel.send("Please create a role called `kulak` and an emoji called `gulag` to use this feature.")
            return True
        comrade = self.message.mentions[0]
        if comrade == self.client.user:
            await self.channel.send(file=discord.File('res/treason.gif'))
            comrade = self.author

        t_end = time.time() + 60 * 30
        user_list = []
        timer_msg = None
        timer_msg_gulag = None
        generated = False
        msg = await self.channel.send("%d more %s's to gulag %s" % (settings.gulag_threshold, gulag_emoji, comrade.display_name))
        await self.client.add_reaction(msg, gulag_emoji)
        while time.time() < t_end:
            res = await self.client.wait_for_reaction(message=msg, emoji=gulag_emoji, timeout=5)
            #print(t_end - time.time())
            if res and res.user not in user_list and res.user != self.client.user:
                user_list.append(res.user) 
                for user in user_list: print (user.display_name)
                await self.client.edit_message(msg, "%d more %s's to gulag %s" % (max(0,(settings.gulag_threshold - len(user_list))), gulag_emoji, comrade.display_name))
                t_end += int((settings.gulag_severity / 2) * 60)
            if len(user_list) >= settings.gulag_threshold and not gulag_role in comrade.roles:
                try:
                    print(comrade.avatar_url if comrade.avatar_url else comrade.default_avatar_url)
                    gulaggen.generate(comrade.avatar_url if comrade.avatar_url else comrade.default_avatar_url)
                    generated = True
                except Exception as e:
                    print(e)
                    pass
                if generated:
                    await self.channel.send(file=discord.File('res/gulag.png'))
                else:
                    await self.channel.send("gulag'd " + comrade.display_name)

                    timer_msg = await self.channel.send("⏰ %d seconds" % (settings.gulag_severity * 60))
                    #TODO
                    timer_msg_gulag = await self.client.send_message(discord.utils.get(server.channels, name='gulag', type=ChannelType.text), "⏰ %d seconds, %s" % (settings.gulag_severity * 60, comrade.display_name))
                    await self.client.add_roles(comrade, gulag_role)

                    if comrade.voice.voice_channel and not comrade.voice.voice_channel.is_private:
                        #TODO
                        try: await self.move_member(comrade, discord.utils.get(self.server.channels, name='gulag', type=ChannelType.voice))
                        except: pass
                    t_end = time.time() + int(60 * settings.gulag_severity)

            elif timer_msg or timer_msg_gulag:
                await self.client.edit_message(timer_msg, "⏰ %d seconds" % (max(0, t_end-time.time())))
                await self.client.edit_message(timer_msg_gulag, "⏰ %d seconds, %s" % (max(0, t_end-time.time()), comrade.display_name))

        await self.client.remove_roles(comrade, gulag_role)
        print('ungulag\'d ' + comrade.display_name)

        return True

    def get_brief(self):
        return "Starts a vote to move a member to the gulag."

    def get_help(self, **kwargs):
        settings = kwargs['settings']
        return "Starts a vote to move a member to the gulag. Each vote over the threshold (%d) will add additional time." % (settings.gulag_threshold if settings else 5)

    def get_usage(self):
        return "<@member>"
