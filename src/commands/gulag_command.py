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
        filtered = filter(lambda role: role.name == "kulak", server.role_hierarchy)
        try:
            gulag_role = next(filtered)
            gulag_emoji = self.get_custom_emoji(server, "gulag")
        except:
            print("gulag role/emoji not found")
            return
        comrade = self.message.mentions[0]
        if comrade == self.client.user:
            with open('res/treason.gif', 'rb') as f:
                await self.client.send_file(self.channel, f)
                comrade = self.author

        t_end = time.time() + 60 * 30
        user_list = []
        timer_msg = None
        timer_msg_gulag = None
        generated = False
        msg = await self.client.send_message(self.channel, "%d more %s's to gulag %s" % (GULAG_THRESHOLD, gulag_emoji, comrade.display_name))
        await self.client.add_reaction(msg, gulag_emoji)
        while time.time() < t_end:
            res = await self.client.wait_for_reaction(message=msg, emoji=gulag_emoji, timeout=5)
            #print(t_end - time.time())
            if res and res.user not in user_list and res.user != self.client.user:
                user_list.append(res.user) 
                for user in user_list: print (user.display_name)
                await self.client.edit_message(msg, "%d more %s's to gulag %s" % (max(0,(GULAG_THRESHOLD - len(user_list))), gulag_emoji, comrade.display_name))
                t_end += GULAG_TIME_ADD * 60
            if len(user_list) >= GULAG_THRESHOLD and not gulag_role in comrade.roles:
                try:
                    print(comrade.avatar_url if comrade.avatar_url else comrade.default_avatar_url)
                    gulaggen.generate(comrade.avatar_url if comrade.avatar_url else comrade.default_avatar_url)
                    generated = True
                except Exception as e:
                    print(e)
                    pass
                with open('res/gulag.png', 'rb') as f:
                    if generated:
                        await self.client.send_file(self.channel, f)
                    else:
                        await self.client.send_message(self.channel, "gulag'd " + comrade.display_name)

                    timer_msg = await self.client.send_message(self.channel, "⏰ %d seconds" % (GULAG_TIME * 60))
                    timer_msg_gulag = await self.client.send_message(discord.utils.get(server.channels, name='gulag', type=ChannelType.text), "⏰ %d seconds, %s" % (GULAG_TIME * 60, comrade.display_name))
                    await self.client.add_roles(comrade, gulag_role)

                    if comrade.voice.voice_channel and not comrade.voice.voice_channel.is_private:
                        try: await self.move_member(comrade, discord.utils.get(self.server.channels, name='gulag', type=ChannelType.voice))
                        except: pass
                    t_end = time.time() + int(60 * GULAG_TIME)

            elif timer_msg or timer_msg_gulag:
                await self.client.edit_message(timer_msg, "⏰ %d seconds" % (max(0, t_end-time.time())))
                await self.client.edit_message(timer_msg_gulag, "⏰ %d seconds, %s" % (max(0, t_end-time.time()), comrade.display_name))

        await self.client.remove_roles(comrade, gulag_role)
        print('ungulag\'d ' + comrade.display_name)

    def get_help(self):
        return "Starts a vote to move a member to the gulag. Each vote over the threshold (%d) will add additional time." % GULAG_THRESHOLD

    def get_usage(self):
        return "<@member>"
