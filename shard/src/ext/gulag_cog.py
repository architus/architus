import src.generate.gulag as gulaggen
from discord.ext import commands
from contextlib import suppress
import time
import asyncio
import discord

from lib.config import logger


class Gulag(commands.Cog):

    def __init__(self, bot):
        self.bot = bot

    @commands.command()
    async def gulag(self, ctx, comrade: discord.Member):
        '''
        Starts a vote to move a member to the gulag.
        Each vote over the threshold will add additional time.
        '''
        settings = self.bot.settings[ctx.guild]
        filtered = filter(lambda role: role.name == "kulak", ctx.guild.roles)
        try:
            gulag_role = next(filtered)
            gulag_emoji = discord.utils.get(ctx.guild.emojis, name="gulag")
            assert gulag_emoji is not None
        except Exception:
            logger.warning("gulag role/emoji not found")
            await ctx.send("Please create a role called `kulak` and an emoji called `gulag` to use this feature.")
            return
        if comrade == self.bot.user:
            await ctx.channel.send(file=discord.File('res/treason.gif'))
            comrade = ctx.author

        t_end = time.time() + 60 * 30
        user_list = []
        timer_msg = None
        timer_msg_gulag = None
        generated = False
        msg = await ctx.send("%d more %s's to gulag %s" % (settings.gulag_threshold, gulag_emoji, comrade.display_name))
        await msg.add_reaction(gulag_emoji)
        while time.time() < t_end:
            user = None

            def check(r, u):
                return r.message.id == msg.id and r.emoji.id == gulag_emoji.id
            with suppress(asyncio.TimeoutError):
                react, user = await self.bot.wait_for('reaction_add', timeout=5, check=check)

            if user and user not in user_list and user != self.bot.user:
                user_list.append(user)
                await msg.edit(content="{0} more {1}'s to gulag {2}".format(
                    max(0, (settings.gulag_threshold - len(user_list))), gulag_emoji, comrade.display_name))
                t_end += int((settings.gulag_severity / 2) * 60)
            if len(user_list) >= settings.gulag_threshold and gulag_role not in comrade.roles:
                try:
                    logger.debug(comrade.avatar_url)
                    img = gulaggen.generate(await comrade.avatar_url_as(format='png', size=1024).read())
                    generated = True
                except Exception:
                    logger.exception("gulag generator error")
                    pass
                if generated:
                    await ctx.channel.send(file=discord.File(img, filename=f'{self.bot.hoarfrost_gen.generate()}.png'))
                else:
                    await ctx.channel.send("gulag'd " + comrade.display_name)

                timer_msg = await ctx.channel.send("⏰ %d seconds" % (settings.gulag_severity * 60))
                timer_msg_gulag = await (discord.utils.get(ctx.guild.text_channels, name='gulag')).send(
                    "⏰ %d seconds, %s" % (settings.gulag_severity * 60, comrade.display_name))
                await comrade.add_roles(gulag_role)

                t_end = time.time() + int(60 * settings.gulag_severity)

            elif timer_msg or timer_msg_gulag:
                await timer_msg.edit(content="⏰ %d seconds" % (max(0, t_end - time.time())))
                await timer_msg_gulag.edit(content="⏰ %d seconds, %s"
                                           % (max(0, t_end - time.time()), comrade.display_name))
        if gulag_role not in comrade.roles:
            await msg.edit(content=f"Vote for {comrade.display_name} failed to pass")

        await comrade.remove_roles(gulag_role)
        logger.info('ungulag\'d ' + comrade.display_name)


def setup(bot):
    bot.add_cog(Gulag(bot))
