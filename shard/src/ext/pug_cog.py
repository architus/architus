from discord.ext import commands
from contextlib import suppress
import time
import asyncio
import discord
from lib.config import logger


class Pug(commands.Cog):

    def __init__(self, bot):
        self.bot = bot

    @commands.command()
    async def startpug(self, ctx, role: discord.Role, requiredPlayers: int):
        '''
        Starts a tally for pugs
        '''
        try:
            pug_emoji = discord.utils.get(ctx.guild.emojis, name="pugger")
            assert pug_emoji is not None
        except Exception:
            logger.warning("pugger emoji not found")
            await ctx.send("Could not find the pugger emoji")
            return

        t_end = time.time() + 60 * 30
        user_list = []
        msg = await ctx.send(f"{requiredPlayers} more {pug_emoji}'s for pugs")
        await msg.add_reaction(pug_emoji)
        while time.time() < t_end:
            user = None

            def check(r, u):
                return r.message.id == msg.id and r.emoji.id == pug_emoji.id
            with suppress(asyncio.TimeoutError):
                react, user = await self.bot.wait_for('reaction_add', timeout=5, check=check)

            if user and user not in user_list and user != self.bot.user:
                user_list.append(user)
                num_left = max(0, (requiredPlayers - len(user_list)))
                await msg.edit(content=f"{num_left} more {pug_emoji}'s for pugs")
                t_end += 600
            if len(user_list) >= requiredPlayers:
                await ctx.channel.send(f"GET ON FOR PUGS {' '.join(map(lambda x: f'<@{x.id}>', user_list))}")
                break

        await msg.edit(content=f"Pugs are {'dead. :cry:' if user_list < requiredPlayers else 'poppin! :fire:'}")

        logger.info('no longer listening for pugs for ' + role.name)


def setup(bot):
    bot.add_cog(Pug(bot))
