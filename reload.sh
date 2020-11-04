#!/bin/bash
containers=$(docker-compose ps | grep -o "architus_shard_")
i=0
while IFS= read -r c; do
	i=$(( i + 1 ))
	echo "$c$i"
	docker cp shard "$c$i":/
	docker exec "$c$i" /bin/sh -c "cp -r /shard/* /app"
	#docker exec "$c$i" /bin/sh -c ls /
done <<< "$containers"
curl -XPOST localhost:5001/coggers/src-ext-play_cog
