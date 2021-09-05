from lib.discord_requests import async_list_guilds_request
from lib.status_codes import StatusCodes as s
from lib.config import logger, which_shard, NUM_SHARDS
from asyncio import create_task, wait

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


async def guild_pool_response(shard_client, partial_event, partial_error, payload, jwt):
    returned_guilds = []

    async def cached_response(shard_id, returned_guilds):
        '''requests information about all the guilds a shard knows about for a user'''
        resp, sc = await shard_client.users_guilds(jwt.id, routing_key=f"shard_rpc_{shard_id}")
        if sc != 200:
            logger.error(f"got bad response from `users_guilds` from shard: {shard_id}")
            await partial_error(
                message=f"shard {shard_id} returned error fetching guilds",
                human="An error occured. Some servers may be temporarily unavailable.",
                context=[resp],
                code=sc)
            return
        for g in resp:
            g.update({'owner': g['owner_id'] == jwt.id})

        # send this shard's chunk of guilds up
        if not payload['finished']:
            copy_payload = payload.copy()
            copy_payload.update({'data': resp})
            returned_guilds += resp
            await partial_event(copy_payload)

    # start tasks for fetching guilds from each shard (but don't await yet)
    shard_tasks = [create_task(cached_response(i, returned_guilds)) for i in range(NUM_SHARDS)]

    # start request to discord for complete list of guilds
    resp, sc = await async_list_guilds_request(jwt)

    # wait for shards to respond before continuing...
    await wait(shard_tasks)

    if sc != s.OK_200:
        logger.error(f"discord returned error from guild_list: {resp}")
        await partial_error(
            message=f"discord returned error fetching guilds",
            human="There was a problem connecting to discord. Some servers may be unavailable.",
            context=[resp],
            code=sc)
        return
    remaining = []
    ids = [g['id'] for g in returned_guilds]
    # merge data from discord and shards to get a complete picture
    for guild in resp:
        if str(guild['id']) not in ids:
            # if it's not in our list from the shards it doesn't have architus

            guild.update({
                'has_architus': False,
                'architus_admin': False,
                'permissions': int(guild['permissions']),
            })
            remaining.append(guild)

    # send the remaining guilds
    payload.update({'data': remaining, 'finished': True})
    await partial_event(payload)


async def pool_response(shard_client, guild_id, pool_type, ids, partial_event, partial_error, payload, jwt):
    resp, sc = await shard_client.pool_request(
        jwt.id, guild_id, pool_type, ids, fetch=False, routing_key=f"shard_rpc_{which_shard(guild_id)}")
    if sc != 200:
        logger.error(f"got bad response from `pool_request` for guild: {guild_id}")
        await partial_error(
            message=f"guild {guild_id} returned error fetching entities of type {pool_type}",
            human=f"An error occured fetching {pool_type}.",
            context=[resp],
            code=sc)
        return
    payload.update({'data': resp['data'], 'finished': len(resp['nonexistant']) == 0})
    if len(payload['data']) > 0:
        await partial_event(payload)
    if payload['finished']:
        return

    resp, sc = await shard_client.pool_request(
        jwt.id, guild_id, pool_type, resp['nonexistant'], fetch=True, routing_key=f"shard_rpc_{which_shard(guild_id)}")
    if sc != 200:
        logger.error(f"got bad response from `pool_request` for guild: {guild_id}")
        await partial_error(
            message=f"guild {guild_id} returned error fetching entities of type {pool_type}",
            human=f"An error occured fetching {pool_type}.",
            context=[resp],
            code=sc)
        return
    payload.update({'data': resp['data'], 'finished': True, 'nonexistant': resp['nonexistant']})
    await partial_event(payload)
