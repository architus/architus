import requests
import json
from lib.config import secret_token

message_url = "https://discordapp.com/api/webhooks/%s/%s"

get_url = "https://discordapp.com/api/channels/%s/webhooks"

create_url = "https://discordapp.com/api/channels/%s/webhooks"
header = {
    "Authorization": "Bot %s" % secret_token,
    "Content-Type": "application/json"
}


def send_message(channel, content, avatar_url=None, username=None, embeds=None):
    requests.post(get_webhook_url(channel), data={
        "content": content, "avatar_url": avatar_url, "username": username, "embeds": embeds
    })


def get_webhooks(channel):
    return json.loads(requests.get(get_url % channel.id, headers=header).text)


def get_webhook_url(channel):
    webhooks = get_webhooks(channel)
    if webhooks:
        webhook = webhooks[0]
        print('reuse')
    else:
        webhook = json.loads(requests.post(
            create_url % channel.id, headers=header, data=json.dumps({"name": channel.name})).text)
        print("make")
    print(message_url % (webhook['id'], webhook['token']))
    return message_url % (webhook['id'], webhook['token'])


def has_webhook(channel):
    return False
