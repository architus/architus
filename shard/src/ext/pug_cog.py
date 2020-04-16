from discord.ext import commands
from contextlib import suppress
import time
import asyncio
from lib.config import logger


class Pug(commands.Cog):

    def __init__(self, bot):
        self.bot = bot

    @commands.command()
    async def startpug(self, ctx, requiredPlayers: int = 10):
        '''
        Starts a tally for pugs
        '''
        settings = self.bot.settings[ctx.guild]
        pug_emoji = settings.pug_emoji

        t_end = time.time() + 60 * 30
        user_list = []
        msg = await ctx.send(f"{requiredPlayers} more {pug_emoji}'s for pugs")
        await msg.add_reaction(pug_emoji)
        while time.time() < t_end:

            def check(r, u):
                return r.message.id == msg.id and str(r.emoji) == pug_emoji

            async def update():
                num_left = max(0, (requiredPlayers - len(user_list)))
                await msg.edit(content=f"{num_left} more {pug_emoji}'s for pugs")

            with suppress(TimeoutError):
                task_add = asyncio.ensure_future(self.bot.wait_for('reaction_add', check=check))
                task_remove = asyncio.ensure_future(self.bot.wait_for('reaction_remove', check=check))
                done, _ = await asyncio.wait([task_add, task_remove], timeout=5, return_when=asyncio.FIRST_COMPLETED)

                if task_add in done:
                    task_remove.cancel()
                    react, user = task_add.result()
                    if user and user not in user_list and user != self.bot.user:
                        user_list.append(user)
                        t_end += int((settings.pug_timeout_speed / 2) * 60)
                        await update()
                elif task_remove in done:
                    task_add.cancel()
                    react, user = task_remove.result()
                    if user and user in user_list and user != self.bot.user:
                        user_list = [u for u in user_list if u.id != user.id]
                        await update()

            if len(user_list) >= requiredPlayers:
                await ctx.channel.send(f"GET ON FOR PUGS {' '.join(map(lambda x: x.mention, user_list))}")
                break

        await msg.edit(content=f"Pugs are {'dead. :cry:' if len(user_list) < requiredPlayers else 'poppin! :fire:'}")

        logger.info(f"no longer listening for pugs for {ctx.message.author}")


def setup(bot):
    bot.add_cog(Pug(bot))
