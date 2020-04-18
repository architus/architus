from discord.ext import commands
from contextlib import suppress
import time
import asyncio
from lib.config import logger


class Pug(commands.Cog):

    def __init__(self, bot):
        self.bot = bot

    @commands.command(aliases=['pug', 'pugs', 'startpugs', 'anypuggers'])
    async def startpug(self, ctx, requiredPlayers: int = 10, game: str = ""):
        '''
        Starts a tally for pugs
        '''
        if requiredPlayers < 1:
            ctx.channel.send("Please specify a playercount greater than 0 :rage:")

        settings = self.bot.settings[ctx.guild]
        pug_emoji = settings.pug_emoji
        pug_timeout_speed = settings.pug_timeout_speed

        t_end = time.time() + 60 * pug_timeout_speed
        user_list = []
        msg = await ctx.send(f"{requiredPlayers} more {pug_emoji}'s for {game}{' ' if game != '' else ''}pugs")
        await msg.add_reaction(pug_emoji)
        while time.time() < t_end:

            def check(r, u):
                return r.message.id == msg.id and str(r.emoji) == pug_emoji

            with suppress(TimeoutError):
                task_add = asyncio.ensure_future(self.bot.wait_for('reaction_add', check=check))
                task_remove = asyncio.ensure_future(self.bot.wait_for('reaction_remove', check=check))
                done, _ = await asyncio.wait([task_add, task_remove], timeout=5, return_when=asyncio.FIRST_COMPLETED)

                if task_add in done:
                    task_remove.cancel()
                    react, user = task_add.result()
                    t_end += int((pug_timeout_speed / 2) * 60)
                elif task_remove in done:
                    task_add.cancel()
                    react, user = task_remove.result()
                else:
                    continue

            user_list = [u for u in await react.users().flatten() if u != self.bot.user]
            num_left = max(0, (requiredPlayers - len(user_list)))
            await msg.edit(content=f"{num_left} more {pug_emoji}'s for pugs")

            if len(user_list) >= requiredPlayers:
                await ctx.channel.send(f"GET ON FOR {game}{' ' if game != '' else ''}PUGS "
                                       f"{' '.join(map(lambda x: x.mention, user_list))}")
                break

        await msg.edit(content=f"Pugs are {'dead. :cry:' if len(user_list) < requiredPlayers else 'poppin! :fire:'}")

        logger.info(f"no longer listening for pugs for {ctx.message.author}")


def setup(bot):
    bot.add_cog(Pug(bot))
