#!/bin/bash

if [ $# -eq 0 ] ; then
    echo "please specify a module to reload (e.g. $0 ext-set_cog)"
	exit -1
fi
port=5000
if [ ! -z "$2" ]; then
	port="$2"
fi
containers=$(docker-compose ps | grep -o "architus_shard_")
i=0
while IFS= read -r c; do
	i=$(( i + 1 ))
	echo "$c$i"
	docker cp shard "$c$i":/
	docker exec "$c$i" /bin/sh -c "cp -r /shard/* /app"
done <<< "$containers"
curl -XPOST "localhost:$port/coggers/src-$1"
