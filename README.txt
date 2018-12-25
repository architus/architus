-----------TO RUN LOCALLY---------------
1. Get your own damn app: https://discordapp.com/developers/applications/me 
3. Install virtualenv and dependencies: 'source install.sh'
4. Activate your virtualenv: 'source .venv/bin/activate'
5. Set up DB:
* Install postgresql
* Create user `autbot`
*  `sudo -u postgres psql -c 'CREATE DATABASE autbot WITH OWNER autbot;'`
*  `sudo -u postgres psql autbot < data/autbot-seed.sql` (you may have to give postgres permission to see this file)
7. Put the discord token, postgres username (`autbot`), and postgres password (from step 4) in a file called `.secret_token`, with a newline between each
8. Run bot: 'python3.6 bot.py'
