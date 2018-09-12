from src.commands.abstract_command import abstract_command
import src.generate.gulag as gulaggen
import time
import discord

class gulag_command(abstract_command):

    def __init__(self):
        super().__init__("gulag")

    async def exec_cmd(self, **kwargs):
        GULAG_THRESHOLD = 1
        GULAG_TIME = .1
        GULAG_TIME_ADD = .1
        server = self.server
        filtered = filter(lambda role: role.name == "kulek", server.role_hierarchy)
        try:
            gulag_role = next(filtered)
            gulag_emoji = self.get_custom_emoji(server, "gulag")
        except:
            print("gulag role/emoji not found")
            return
        comrade = self.message.mentions[0]
        t_end = time.time() + 60 * 30
        user_list = []
        timer_msg = None
        timer_msg_gulag = None
        msg = await self.client.send_message(self.channel, "%d more %s's to gulag %s" % (GULAG_THRESHOLD, gulag_emoji, comrade.mention))
        await self.client.add_reaction(msg, gulag_emoji)
        while time.time() < t_end:
            res = await self.client.wait_for_reaction(message=msg, emoji=gulag_emoji, timeout=5)
            print(t_end - time.time())
            if res and res.user not in user_list and res.user != self.client.user:
                user_list.append(res.user) 
                await self.client.edit_message(msg, "%d more %s's to gulag %s" % (max(0,(GULAG_THRESHOLD - len(user_list))), gulag_emoji, comrade.mention))
                t_end += GULAG_TIME_ADD * 60
            if len(user_list) >= GULAG_THRESHOLD and not gulag_role in comrade.roles:
                gulaggen.generate(comrade.avatar_url if comrade.avatar_url else comrade.default_avatar_url)
                with open('res/gulag.png', 'rb') as f:
                    await self.client.send_file(self.channel, f)
                    timer_msg = await self.client.send_message(self.channel, "⏰ %d seconds" % (GULAG_TIME * 60))
                    timer_msg_gulag = await self.client.send_message(self.get_channel_by_name(server,'gulag')[0], "⏰ %d seconds, %s" % (GULAG_TIME * 60, comrade.mention))
                    await self.client.add_roles(comrade, gulag_role)
                    t_end = time.time() + int(60 * GULAG_TIME)
            elif timer_msg or timer_msg_gulag:
                await self.client.edit_message(timer_msg, "⏰ %d seconds" % (max(0, t_end-time.time())))
                await self.client.edit_message(timer_msg_gulag, "⏰ %d seconds, %s" % (max(0, t_end-time.time()), comrade.mention))

        await self.client.remove_roles(comrade, gulag_role)
        print('ungulag\'d ' + comrade.display_name)

    def get_help(self):
        return "!gulag <@member> - hold a gulag vote"

    def get_usage(self):
        return "!gulag <@member> - hold a gulag vote"
