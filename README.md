# Architus
[![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Farchitus%2Farchitus.svg?type=shield)](https://app.fossa.io/projects/git%2Bgithub.com%2Farchitus%2Farchitus?ref=badge_shield)


A multipurpose discord bot implemented using the discord.py library.

### Features
* Web-interface
* Custom responses
* Role management
* Music
* Starboard
* Events

### Installing
1. Create a discord app: https://discordapp.com/developers/applications/me 
3. Install virtualenv and dependencies: 'source install.sh'
4. Activate your virtualenv: 'source .venv/bin/activate'
5. Set up DB:
* Install postgresql
* Create user `autbot`
*  `sudo -u postgres psql -c 'CREATE DATABASE autbot WITH OWNER autbot;'`
*  `sudo -u postgres psql autbot < data/autbot-seed.sql` (you may have to give postgres permission to see this file)
7. Put the discord token, postgres username (`autbot`), and postgres password (from step 4) in a file called `.secret_token`, on separate lines
8. Run bot: 'python3.6 bot.py'


## License
[![FOSSA Status](https://app.fossa.io/api/projects/git%2Bgithub.com%2Farchitus%2Farchitus.svg?type=large)](https://app.fossa.io/projects/git%2Bgithub.com%2Farchitus%2Farchitus?ref=badge_large)