from discord.ext import commands
from src.user_command import UserCommand, VaguePatternError, LongResponseException, ShortTriggerException, ResponseKeywordException, DuplicatedTriggerException, update_command
from src.config import get_session
import re
import discord


class SetCog(commands.Cog, name="Auto Responses"):

    def __init__(self, bot):
        self.bot = bot
        self.session = get_session()

    @property
    def guild_settings(self):
        return self.bot.get_cog('GuildSettings')

    @commands.command()
    async def remove(self, ctx, trigger):
        '''Remove a user command.'''
        msg = 'no command with that trigger'
        for oldcommand in self.bot.user_commands[ctx.guild.id]:
            if oldcommand.raw_trigger == oldcommand.filter_trigger(trigger):
                self.bot.user_commands[ctx.guild.id].remove(oldcommand)
                update_command(self.session, oldcommand.raw_trigger, '', 0, ctx.guild, ctx.author.id, delete=True)
                msg = 'removed `' + oldcommand.raw_trigger + "::" + oldcommand.raw_response + '`'
        await ctx.channel.send(msg)

    def validate(self, guild_id, command):
        return not any(command == oldcommand for oldcommand in self.bot.user_commands[guild_id])\
            and not len(command.raw_trigger) == 0 and command.raw_response not in ['remove', 'author']

    @commands.command()
    async def set(self, ctx, *args):
        '''
        Sets a custom command
        You may include the following options:
        [noun], [adj], [adv], [member], [owl], [:reaction:], [count], [comma,separated,choices]
        '''
        user_commands = self.bot.user_commands
        settings = self.guild_settings.get_guild(ctx.guild, session=self.session)
        from_admin = ctx.author.id in settings.admins_ids
        if settings.bot_commands_channels and ctx.channel.id not in settings.bot_commands_channels and not from_admin:
            for channelid in settings.bot_commands_channels:
                botcommands = discord.utils.get(ctx.guild.channels, id=channelid)
                if botcommands:
                    await ctx.channel.send(botcommands.mention + '?')
                    return

        parser = re.search('!set (.+?)::(.+)', ctx.message.content, re.IGNORECASE)
        msg = "try actually reading the syntax"
        if parser:
            try:
                command = UserCommand(self.session, parser.group(1), parser.group(2), 0, ctx.guild, ctx.author.id, new=True)
            except VaguePatternError:
                msg = "let's try making that a little more specific please"
            except (LongResponseException, ShortTriggerException) as e:
                msg = str(e)
            except ResponseKeywordException:
                if parser.group(2).strip() == "remove":
                    msg = "please use `!remove` instead"
                elif parser.group(2).strip() in ("author", "list"):
                    msg = f"please check https://aut-bot.com/app/{ctx.guild.id}/responses"
            except DuplicatedTriggerException as e:
                msg = "That's a dupe idiot"
            else:
                user_commands[ctx.guild.id].append(command)
                msg = 'command set'

        await ctx.channel.send(msg)


def setup(bot):
    bot.add_cog(SetCog(bot))
