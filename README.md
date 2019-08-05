# [![architus](https://i.imgur.com/qfPmMBW.png)](https://archit.us)

[![FOSSA Status](https://app.fossa.com/api/projects/git%2Bgithub.com%2Farchitus%2Farchitus.svg?type=shield)](https://app.fossa.com/projects/git%2Bgithub.com%2Farchitus%2Farchitus?ref=badge_shield) [![Build Status](https://travis-ci.org/architus/architus.svg?branch=develop)](https://travis-ci.org/architus/archit.us) [![API Uptime](https://img.shields.io/uptimerobot/ratio/7/m782992399-3443671051db8aeaecfe7434.svg?label=API%20uptime)](https://status.archit.us/)
[![Discord Server](https://img.shields.io/discord/607637793107345431?color=7289DA&logo=discord&logoColor=white)](https://discord.gg/FpyhED)


> Architus is a multi-purpose Discord bot implemented using the discord.py library that empowers both admins and server members with the tools and features to have a more streamlined and enjoyable experience.

## Features

* [Web-interface](https://archit.us/app)
* Custom responses
* Role management
* Music
* Starboard
* Events

## Invite

You can invite architus to your discord server with [this link](https://api.archit.us/invite/0) or through the web-interface.

## Installing

1. Create a discord app: https://discordapp.com/developers/applications/me 
3. Install virtualenv and dependencies: `source install.sh`
4. Activate your virtualenv: `source .venv/bin/activate`
5. Set up DB:
* Install postgresql
* Create user `autbot`
*  `sudo -u postgres psql -c 'CREATE DATABASE autbot WITH OWNER autbot;'`
*  `sudo -u postgres psql autbot < data/autbot-seed.sql` (you may have to give postgres permission to see this file)
7. Put the discord token, postgres username (`autbot`), and postgres password (from step 4) in a file called `.secret_token`, on separate lines
8. Run bot: `python3.6 bot.py`

## License
[![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Farchitus%2Farchitus.svg?type=large)](https://app.fossa.io/projects/git%2Bgithub.com%2Farchitus%2Farchitus?ref=badge_large)
