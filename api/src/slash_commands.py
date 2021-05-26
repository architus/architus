from flask import request
from json import loads
from discord_interactions import verify_key_decorator, InteractionType, InteractionResponseType

from lib.discord_requests import register_command, register_guild_command
from lib.config import logger, application_public_key
from src.util import CustomResource


commands = []
with open('./src/slash_commands/set.json') as f:
    commands.append(loads(f.read()))


def init():
    logger.info("Registering slash commands")
    for c in commands:
        logger.debug(register_command(c))
        logger.debug(register_guild_command(436189230390050826, c))


class DiscordInteraction(CustomResource):
    @verify_key_decorator(application_public_key)
    def post(self):
        if request.json['type'] == InteractionType.APPLICATION_COMMAND:
            return {
                'type': InteractionResponseType.CHANNEL_MESSAGE_WITH_SOURCE,
                'data': {
                    'content': 'Hello world'
                }
            }, 200
