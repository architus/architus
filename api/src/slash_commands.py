from lib.status_codes import StatusCodes
from flask import request
import os
from json import loads
from discord_interactions import verify_key_decorator, InteractionType, InteractionResponseType

from lib.discord_requests import register_command
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


class DiscordInteraction(CustomResource):
    @verify_key_decorator(application_public_key)
    def post(self):
        sc = StatusCodes.NOT_FOUND_404
        data = request.json
        options = data['data']['options']
        guild_id = data['guild_id']
        channel_id = int(data['channel_id'])
        if data['type'] == InteractionType.APPLICATION_COMMAND:
            command = data['data']['name']
            member_id = data['member']['user']['id']

            if command == 'set':
                trigger = next(o['value'] for o in options if o['name'] == 'trigger')
                response = next(o['value'] for o in options if o['name'] == 'response')
                reply = next((o['value'] for o in options if o['name'] == 'reply'), False)
                resp, sc = self.shard.set_response(
                    guild_id, member_id, trigger, response, reply, routing_guild=guild_id)
            elif command == 'remove':
                trigger = next(o['value'] for o in options if o['name'] == 'trigger')
                resp, sc = self.shard.remove_response(guild_id, member_id, trigger, routing_guild=guild_id)
            elif command == 'role-setup':
                emoji = {o['name']: o['value'] for o in options if o['name'].startswith('emoji')}
                roles = {o['name']: o['value'] for o in options if o['name'].startswith('role')}
                both = {value: roles['role' + name.replace('emoji', '')] for name, value in emoji.items()}
                resp, sc = self.shard.role_setup(guild_id, channel_id, member_id, both, routing_guild=guild_id)
            return {
                'type': InteractionResponseType.CHANNEL_MESSAGE_WITH_SOURCE,
                'data': {
                    'content': resp['content']
                }
            }, sc
