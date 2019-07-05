from discord.ext import commands
from src.models import Command
from src.user_command import UserCommand, VaguePatternError, update_command
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
        if parser and (len(parser.group(2)) <= 200 and len(parser.group(1)) > 1 and ctx.guild.default_role.mention
                       not in parser.group(2) or from_admin):
            try:
                command = UserCommand(parser.group(1), parser.group(2), 0, ctx.guild, ctx.author.id)
            except VaguePatternError:
                await self.channel.send("let's try making that a little more specific please")
                return

            if self.validate(ctx.guild.id, command):
                user_commands[ctx.guild.id].append(command)
                if ctx.guild.id > 1000000000:
                    new_command = Command(str(ctx.guild.id) + command.raw_trigger, command.raw_response,
                                          command.count, ctx.guild.id, ctx.author.id)
                    self.session.add(new_command)
                    self.session.commit()
                msg = 'command set'
            elif parser.group(2).strip() == "remove":
                msg = 'no command with that trigger'
                for oldcommand in user_commands[ctx.guild.id]:
                    if oldcommand == command:
                        user_commands[ctx.guild.id].remove(oldcommand)
                        update_command(self.session, oldcommand.raw_trigger, '', 0,
                                       ctx.guild, ctx.author.id, delete=True)
                        msg = 'removed `' + oldcommand.raw_trigger + "::" + oldcommand.raw_response + '`'
                        msg += "\nthis syntax is deprecated and will go away soon."
                        msg += f" try using `!remove {oldcommand.raw_trigger}` next time"
            elif parser.group(2).strip() == "list":
                for oldcommand in user_commands[ctx.guild.id]:
                    if oldcommand == command:
                        msg = f"`{oldcommand}`"
                        break
                else:
                    msg = 'no command with that trigger'
            elif parser.group(2).strip() == "author":
                msg = 'no command with that trigger'
                for oldcommand in user_commands[ctx.guild.id]:
                    if oldcommand == command:
                        try:
                            usr = await self.bot.fetch_user(int(oldcommand.author_id))
                            msg = usr.name + '#' + usr.discriminator
                        except Exception:
                            msg = 'idk'
            else:
                for oldcommand in user_commands[ctx.guild.id]:
                    if oldcommand == command:
                        msg = f'Remove `{oldcommand.raw_trigger}` first (`!remove`)'
        elif parser and len(parser.group(2)) >= 200:
            msg = 'too long, sorry. ask an admin to set it'
        elif parser and len(parser.group(1)) > 1:
            msg = 'too short'
        await ctx.channel.send(msg)


def setup(bot):
    bot.add_cog(SetCog(bot))
