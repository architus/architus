from lib.discord_requests import async_list_guilds_request
from lib.status_codes import StatusCodes as s
from lib.config import logger, which_shard, NUM_SHARDS
from asyncio import create_task

guild_attrs = [
    'id', 'name', 'icon', 'splash', 'owner_id', 'region', 'description',
    'afk_timeout', 'unavailable', 'max_members', 'banner',
    'mfa_level', 'premium_tier', 'premium_subscription_count',
    'preferred_locale', 'member_count'
]


def guilds_to_dicts(guilds):
    for g in guilds:
        guild_dict = dict()
        for attr in guild_attrs:
            value = getattr(g, attr)
            if type(value) == int and value == 0:
                value = None
            if type(value) == str and value == '':
                value = None
            guild_dict[attr] = value
        guild_dict['features'] = list()
        for feat in g.features:
            guild_dict['features'].append(str(feat))
        # javascript requires numbers to be strings for some odd reason
        guild_dict['admin_ids'] = list(map(str, g.admin_ids))
        # TODO Remove this when joey fixes the frontend
        guild_dict['region'] = [guild_dict['region']]
        yield guild_dict


class GuildPool:
    def __init__(self, manager_client, shard_client, jwt):
        self.manager_client = manager_client
        self.shard_client = shard_client
        self.jwt = jwt
        self.return_guilds = []
        self.list_task = None

    async def fetch_guilds(self):
        resp, sc = await async_list_guilds_request(self.jwt)
        if sc != s.OK_200:
            logger.debug(resp)
            return []

        guilds = []

        for guild in resp:
            mem_resp, _ = await self.shard_client.is_member(
                self.jwt.id, guild['id'], routing_key=f"shard_rpc_{which_shard(guild['id'])}")
            guild.update({
                'has_architus': mem_resp['member'],
                'architus_admin': mem_resp['admin'],
            })
            guilds.append(guild)

        return guilds

    async def fetch_architus_guilds(self):
        # get discord request started
        self.list_task = create_task(async_list_guilds_request(self.jwt))

        tasks = (create_task(
            self.shard_client.users_guilds(self.jwt.id, routing_key=f"shard_rpc_{i}"))
            for i in range(NUM_SHARDS))

        self.return_guilds = [guild for t in tasks for guild in (await t)[0]]
        for g in self.return_guilds:
            g.update({'owner': g['owner_id'] == self.jwt.id})
        return self.return_guilds

    async def fetch_remaining_guilds(self):
        resp, sc = await self.list_task
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
