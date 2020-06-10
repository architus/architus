from lib.discord_requests import async_list_guilds_request
from lib.status_codes import StatusCodes as s
from lib.config import logger, which_shard
import lib.ipc.manager_pb2 as message

guild_attrs = [
    'shard_id', 'id', 'name', 'icon', 'splash', 'owner_id', 'region',
    'afk_timeout', 'unavailable', 'max_members', 'banner', 'description',
    'mfa_level', 'premium_tier', 'premium_subscription_count',
    'preferred_locale', 'member_count'
]

def guilds_to_dicts(guilds):
    for g in guilds:
        guild_dict = dict()
        for attr in guild_attrs:
            guild_dict[attr] = getattr(g, attr)
        guild_dict['features'] = list()
        for feat in g.features:
            guild_dict['features'].append(str(feat))
        # javascript requires numbers to be strings for some odd reason
        guild_dict['id'] = str(guild_dict['id'])
        guild_dict['admin_ids'] = map(lambda id: str(id), g.admin_ids)
        yield guild_dict

class GuildPool:
    def __init__(self, manager_client, shard_client, jwt):
        self.manager_client = manager_client
        self.shard_client = shard_client
        self.jwt = jwt
        self.return_guilds = []

    async def fetch_architus_guilds(self):
        all_guilds_message = await self.manager_client.all_guilds(message.AllGuildsRequest())
        all_guilds = guilds_to_dicts(all_guilds_message)
        for guild in all_guilds:
            resp, _ = await self.shard_client.is_member(
                self.jwt.id, guild['id'], routing_key=f"shard_rpc_{which_shard(guild['id'])}")
            if resp['member']:
                guild.update({
                    'id': str(guild['id']),
                    'has_architus': True,
                    'architus_admin': resp['admin'],
                    'owner': guild['owner_id'] == self.jwt.id,
                    'permissions': resp['permissions']
                })
                del guild['owner_id']
                del guild['admin_ids']
                self.return_guilds.append(guild)

        return self.return_guilds

    async def fetch_remaining_guilds(self):
        resp, sc = await async_list_guilds_request(self.jwt)
        if sc != s.OK_200:
            logger.debug(resp)
            return []
        remaining = []
        ids = [g['id'] for g in self.return_guilds]
        for guild in resp:
            if str(guild['id']) not in ids:
                guild.update({
                    'has_architus': False,
                    'architus_admin': False,
                })
                remaining.append(guild)

        return remaining
