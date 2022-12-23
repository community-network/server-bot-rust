FROM rust:1.62 as builder
WORKDIR /usr/src/myapp
COPY . .
ARG github_token 
RUN git config --global credential.helper store && echo "https://zefanjajobse:${github_token}@github.com" > ~/.git-credentials && cargo install --path .

FROM debian:bullseye

ENV token default_token_value
ENV name default_name_value
ENV channel default_channel_value
ENV minplayeramount '20'
ENV prevrequestcount '5'
ENV startedamount '50'
ENV guild default_guild_value
ENV lang 'en-us'
ENV guid 'false'
ENV ownerId 'none'

HEALTHCHECK --interval=5m --timeout=3s --start-period=5s \
  CMD curl -f http://127.0.0.1:3030/ || exit 1

COPY --from=builder /usr/local/cargo/bin/discord_bot /usr/local/bin/discord_bot
RUN apt-get update && apt-get install --assume-yes curl
CMD ["discord_bot"]
