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


async def guild_pool_response(shard_client, partial_event, partial_error, payload, jwt):
    returned_guilds = []

    async def cached_response(shard_id, returned_guilds):
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
        if not payload['finished']:
            payload.update({'data': resp})
            returned_guilds += resp
            await partial_event(payload)

    for i in range(NUM_SHARDS):
        create_task(cached_response(i, returned_guilds))

    resp, sc = await async_list_guilds_request(jwt)
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
    for guild in resp:
        if str(guild['id']) not in ids:
            mem_resp, sc = await shard_client.is_member(
                jwt.id, guild['id'], routing_key=f"shard_rpc_{which_shard(guild['id'])}")
            guild.update({
                'has_architus': mem_resp['member'] if sc == 200 else False,
                'architus_admin': mem_resp['admin'] if sc == 200 else False,
            })
            remaining.append(guild)
    payload.update({'data': remaining, 'finished': True})
    await partial_event(payload)


async def pool_response(shard_client, guild_id, pool_type, ids, partial_event, partial_error, payload, jwt):
    return_data = []
    not_cached = []

    shard_id = which_shard(guild_id)

    async def shard_request(entity, fetch):
        resp, sc = await shard_client.pool_request(
            guild_id, pool_type, entity, fetch=fetch, routing_key=f"shard_rpc_{shard_id}")
        if sc != 200:
            logger.error(f"got bad response from `pool_request` from shard: {shard_id}")
            await partial_error(
                message=f"shard {shard_id} returned error fetching entity {entity} of type {pool_type}",
                human="An error occured fetching {pool_type}.",
                context=[resp],
                code=sc)
            return
        if resp['data']:
            return_data.append(resp['data'])
        else:
            not_cached.append(entity)
            await partial_event(payload)

    tasks = [create_task(shard_request(id, False)) for id in ids]
    for t in tasks:
        await t
    payload.update({'data': return_data, 'finished': len(not_cached) == 0})
    if len(return_data) > 0:
        await partial_event(payload)
    if payload['finished']:
        return

    return_data = []
    tasks = [create_task(shard_request(id, True)) for id in not_cached]
    for t in tasks:
        await t
    payload.update(
        {'data': return_data, 'finished': True, 'nonexistant': [i for i in not_cached if i not in return_data]})
    await partial_event(payload)
