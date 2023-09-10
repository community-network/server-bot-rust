# Discord status bots for battlefield

This bot shows info about your battlefield servers and updates it every 2 minutes, this bot can also send updates when the status of the server changes. same as the python version, but better when needing running 50 at a time.

### Environment items:

```yaml
guid: serverid (optional)
game: game name (tunguska, casablanca, kingston, bf4)
ownerId: server owner playerid (optional for casablanca and kingston)
fakeplayers: removes bots on bf4's playercount if set to yes (optional, default no)
name: servername
lang: language (default en-us)
platform: from which platform is the server (default pc)

for status in discord:
minplayeramount: amount of change needed to count
prevrequestcount: amount of request to use for the calculation if the difference is more than min_player_amount
channel: channel where it needs to post the message if almost empty etc.
startedamount: amount of players before it calls the server "started"
```

run with

```bash
export token=TOKEN
export name=SERVERNAME
export lang=en-us
export minplayeramount=20
export prevrequestcount=5
export channel=0
export startedamount=50
export game=tunguska
export platform=pc
cargo run
```

Or use docker:

```docker
version: '3.7'

services:
    ace-bot-1:
      image: ghcr.io/community-network/server-bot-rust/server-bot-rust:latest
      restart: always
      environment:
        - token=TOKEN
        - name=[ACE]#1
        - platform=pc
        - channel=0
        - minplayeramount=20
        - prevrequestcount=5
        - startedamount=50
        - guild=0
        - game=tunguska
        - lang=en-us
      healthcheck:
        test: ["CMD", "curl", "-f", "http://127.0.0.1:3030/"]
        interval: "60s"
        timeout: "3s"
        start_period: "5s"
        retries: 3
```

Or on windows: (.bat file)

```
@ECHO OFF
SET token=DISCORDTOKEN
SET name=SUPER@ [SiC] S1
SET game=bf2
SET lang=en-us
SET minplayeramount=20
SET prevrequestcount=5
SET channel=0
SET startedamount=50

discord_bot.exe
```

This initially used the game api directly, but to not login to the api constandly (many groups use this, so could block logins) it was made to use our main api. it uses the codename for the game for backwards compatability with all locations it is used.

### Game names:

"tunguska" = Battlefield 1

"casablanca" = Battlefield V

"kingston" = Battlefield 2042

#### API Documentation:

- [api.gametools.network](https://api.gametools.network/docs)

#### example images:

![messages send by bot](https://media.discordapp.net/attachments/722532776523464725/828958877071966267/unknown.png)

![serverinfo bots example](https://cdn.discordapp.com/attachments/722532776523464725/828955160336269332/unknown.png)
