from discord.ext.commands.formatter import HelpFormatter

class BetterHelpFormatter(HelpFormatter):
    """The default base implementation that handles formatting of the help
    command.
    To override the behaviour of the formatter, :meth:`format`
    should be overridden. A number of utility functions are provided for use
    inside that method.
    Parameters
    -----------
    show_hidden : bool
        Dictates if hidden commands should be shown in the output.
        Defaults to ``False``.
    show_check_failure : bool
        Dictates if commands that have their :attr:`Command.checks` failed
        shown. Defaults to ``False``.
    width : int
        The maximum number of characters that fit in a line.
        Defaults to 80.
    """
    def __init__(self, show_hidden=False, show_check_failure=False, width=80):
        super(BetterHelpFormatter, self).__init__(show_hidden=show_hidden, show_check_failure=show_check_failure, width=width)



    def format_help_for(self, context, command_or_bot):
        """Formats the help page and handles the actual heavy lifting of how
        the help command looks like. To change the behaviour, override the
        :meth:`format` method.
        Parameters
        -----------
        context : :class:`Context`
            The context of the invoked help command.
        command_or_bot : :class:`Command` or :class:`Bot`
            The bot or command that we are getting the help of.
        Returns
        --------
        list
            A paginated output of the help command.
        """
        self.context = context
        self.command = command_or_bot
        return self.format()

    def format(self):
        """Handles the actual behaviour involved with formatting.
        To change the behaviour, this method should be overridden.
        Returns
        --------
        list
            A paginated output of the help command.
        """
        print("hello")
        self._paginator = Paginator()

        # we need a padding of ~80 or so

        description = self.command.description if not self.is_cog() else inspect.getdoc(self.command)

        if description:
            # <description> portion
            self._paginator.add_line(description, empty=True)

        if isinstance(self.command, Command):
            # <signature portion>
            signature = self.get_command_signature()
            self._paginator.add_line(signature, empty=True)

            # <long doc> section
            if self.command.help:
                self._paginator.add_line(self.command.help, empty=True)

            # end it here if it's just a regular command
            if not self.has_subcommands():
                self._paginator.close_page()
                return self._paginator.pages

        max_width = self.max_name_size

        def category(tup):
            cog = tup[1].cog_name
            # we insert the zero width space there to give it approximate
            # last place sorting position.
            return cog + ':' if cog is not None else '\u200bNo Category:'

        if self.is_bot():
            data = sorted(self.filter_command_list(), key=category)
            for category, commands in itertools.groupby(data, key=category):
                # there simply is no prettier way of doing this.
                commands = list(commands)
                if len(commands) > 0:
                    self._paginator.add_line(category)

                self._add_subcommands_to_page(max_width, commands)
        else:
            self._paginator.add_line('Commands:')
            self._add_subcommands_to_page(max_width, self.filter_command_list())

        # add the ending note
        self._paginator.add_line()
        ending_note = self.get_ending_note()
        self._paginator.add_line(ending_note)
        return self._paginator.pages
