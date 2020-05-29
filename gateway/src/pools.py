from lib.discord_requests import async_list_guilds_request
from lib.status_codes import StatusCodes as s
from lib.config import logger, which_shard


class GuildPool:

    def __init__(self, manager_client, shard_client, jwt):
        self.manager_client = manager_client
        self.shard_client = shard_client
        self.jwt = jwt
        self.return_guilds = []

    async def fetch_architus_guilds(self):

        all_guilds, _ = await self.manager_client.all_guilds()
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
