# Discord status bots for battlefield
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
        - channel=0
        - minplayeramount=20
        - prevrequestcount=5
        - startedamount=50
        - guild=0
        - lang=en-us
      healthcheck:
        test: ["CMD", "curl", "-f", "http://127.0.0.1:3030/"]
        interval: "60s"
        timeout: "3s"
        start_period: "5s"
        retries: 3
```