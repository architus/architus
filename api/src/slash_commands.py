from flask import request
import os
from json import loads
from discord_interactions import verify_key_decorator, InteractionType, InteractionResponseType

from lib.discord_requests import register_command, register_guild_command
from lib.config import logger, application_public_key
from src.util import CustomResource


commands = []
for name in os.listdir('./src/slash_commands/'):
    with open('./src/slash_commands/' + name) as f:
        commands.append(loads(f.read()))


def init():
    logger.info("Registering slash commands")
    for c in commands:
        logger.debug(register_command(c))
        logger.debug(register_guild_command(436189230390050826, c))


class DiscordInteraction(CustomResource):
    @verify_key_decorator(application_public_key)
    def post(self):
        data = request.json
        options = data['data']['options']
        if data['type'] == InteractionType.APPLICATION_COMMAND:
            trigger = next(o['value'] for o in options if o['name'] == 'trigger')
            if data['data']['name'] == 'set':
                response = next(o['value'] for o in options if o['name'] == 'response')
                reply = next((o['value'] for o in options if o['name'] == 'reply'), False)
                resp, _ = self.shard.set_response(
                    data['guild_id'], data['member']['user']['id'], trigger, response, reply)

            else:
                resp, _ = self.shard.remove_response(data['guild_id'], data['member']['user']['id'], trigger)
            return {
                'type': InteractionResponseType.CHANNEL_MESSAGE_WITH_SOURCE,
                'data': {
                    'content': resp['content']
                }
            }, 200
