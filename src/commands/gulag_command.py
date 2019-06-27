import src.generate.gulag as gulaggen
from discord.ext import commands
import time
import discord

class Gulag(commands.Cog):

    def __init__(self, bot):
        self.bot = bot

    @property
    def guild_settings(self):
        return self.bot.get_cog('GuildSettings')

    @commands.command()
    async def gulag(self, ctx, comrade: discord.Member):
        '''Starts a vote to move a member to the gulag. Each vote over the threshold will add additional time.'''
        server = ctx.message.guild
        settings = self.guild_settings.get_guild(server)
        filtered = filter(lambda role: role.name == "kulak", server.roles)
        try:
            gulag_role = next(filtered)
            gulag_emoji = self.get_custom_emoji(server, "gulag")
        except:
            print("gulag role/emoji not found")
            await ctx.channel.send("Please create a role called `kulak` and an emoji called `gulag` to use this feature.")
            return
        #comrade = self.message.mentions[0]
        if comrade == self.bot.user:
            await ctx.channel.send(file=discord.File('res/treason.gif'))
            comrade = ctx.author

        t_end = time.time() + 60 * 30
        user_list = []
        timer_msg = None
        timer_msg_gulag = None
        generated = False
        msg = await ctx.channel.send("%d more %s's to gulag %s" % (settings.gulag_threshold, gulag_emoji, comrade.display_name))
        await msg.add_reaction(gulag_emoji)
        while time.time() < t_end:
            res = await self.bot.wait_for('reaction', timeout=5, check=lambda r,u: r.message == msg and r.emoji == gulag_emoji)
            #print(t_end - time.time())
            if res and res.user not in user_list and res.user != self.bot.user:
                user_list.append(res.user)
                for user in user_list: print (user.display_name)
                await msg.edit("%d more %s's to gulag %s" % (max(0,(settings.gulag_threshold - len(user_list))), gulag_emoji, comrade.display_name))
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
                    await ctx.channel.send(file=discord.File('res/gulag.png'))
                else:
                    await ctx.channel.send("gulag'd " + comrade.display_name)

                    timer_msg = await self.channel.send("⏰ %d seconds" % (settings.gulag_severity * 60))
                    #TODO
                    timer_msg_gulag = await (discord.utils.get(server.text_channels, name='gulag')).send("⏰ %d seconds, %s" % (settings.gulag_severity * 60, comrade.display_name))
                    await comrade.add_roles(gulag_role)

                    if comrade.voice.voice_channel and not comrade.voice.voice_channel.is_private:
                        #TODO
                        try: await self.move_member(comrade, discord.utils.get(self.server.voice_channels, name='gulag'))
                        except: pass
                    t_end = time.time() + int(60 * settings.gulag_severity)

            elif timer_msg or timer_msg_gulag:
                await timer_msg.edit("⏰ %d seconds" % (max(0, t_end-time.time())))
                await timer_msg_gulag.edit("⏰ %d seconds, %s" % (max(0, t_end-time.time()), comrade.display_name))

        await comrade.remove_roles(gulag_role)
        print('ungulag\'d ' + comrade.display_name)

def setup(bot):
    bot.add_cog(Gulag(bot))
