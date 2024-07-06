use anyhow::Result;
use chrono::Utc;
use serenity::{
    builder::{CreateAttachment, EditProfile},
    client::{Client, Context, EventHandler},
    model::gateway::Ready,
    prelude::GatewayIntents,
};
use std::{
    ops::Add,
    sync::{atomic, Arc},
    {env, time},
};
use warp::Filter;
mod message;
mod server_info;

struct Handler;

#[serenity::async_trait]
impl EventHandler for Handler {
    async fn ready(&self, ctx: Context, _: Ready) {
        let user = ctx.cache.current_user().clone();
        log::info!("Logged in as {:#?}", user.name);

        let last_update = Arc::new(atomic::AtomicI64::new(0));
        let last_update_clone = Arc::clone(&last_update);

        let mut message_globals = message::Global {
            game_id: String::from(""),
            since_empty: false,
            previous_request: Vec::new(),
            since_player_trigger: 5,
        };

        let statics = message::Static {
            server_name: env::var("name")
                .expect("name wasn't given an argument!")
                .replace('`', "#")
                .replace('*', "\\\""),
            // optional:
            server_id: env::var("guid").unwrap_or_else(|_| "none".to_string()),
            game: env::var("game").unwrap_or_else(|_| "tunguska".to_string()),
            owner_id: env::var("ownerId").unwrap_or_else(|_| "none".to_string()),
            platform: env::var("platform").unwrap_or_else(|_| "pc".to_string()),
            fake_players: env::var("fakeplayers").unwrap_or_else(|_| "no".to_string()),
            set_banner_image: env::var("serverbanner").unwrap_or_else(|_| "yes".to_string()),
            lang: env::var("lang")
                .unwrap_or_else(|_| "en-us".to_string())
                .to_lowercase(),
            mins_between_avatar_change: env::var("mins_between_avatar_change")
                .unwrap_or_else(|_| "1".to_string())
                .parse::<i32>()
                .expect("mins_between_avatar_change wasn't given an integer!"),
            started_amount: env::var("startedamount")
                .unwrap_or_else(|_| "50".to_string())
                .parse::<i32>()
                .expect("startedamount wasn't given an integer!"),
            message_channel: env::var("channel")
                .unwrap_or_else(|_| "default_channel_value".to_string())
                .parse::<u64>()
                .expect("channel wasn't given an integer!"),
            min_player_amount: env::var("minplayeramount")
                .unwrap_or_else(|_| "20".to_string())
                .parse::<i32>()
                .expect("I wasn't given an integer!"),
            amount_of_prev_request: env::var("prevrequestcount")
                .unwrap_or_else(|_| "5".to_string())
                .parse::<i32>()
                .expect("prevrequestcount wasn't given an integer!"),
        };

        log::info!("Started monitoring server {:#?}", statics.server_name);

        tokio::spawn(async move {
            let hello = warp::any().map(move || {
                let last_update_i64 = last_update_clone.load(atomic::Ordering::Relaxed);
                let now_minutes = Utc::now().timestamp() / 60;
                if (now_minutes - last_update_i64) > 5 {
                    warp::reply::with_status(
                        format!("{}", now_minutes - last_update_i64),
                        warp::http::StatusCode::SERVICE_UNAVAILABLE,
                    )
                } else {
                    warp::reply::with_status(
                        format!("{}", now_minutes - last_update_i64),
                        warp::http::StatusCode::OK,
                    )
                }
            });
            warp::serve(hello).run(([0, 0, 0, 0], 3030)).await;
        });

        // loop in separate async
        tokio::spawn(async move {
            // set update_avatar to 1 minute ago to allow changing on startup
            let mut update_avatar = chrono::Utc::now()
                - chrono::Duration::minutes(statics.mins_between_avatar_change.into());
            loop {
                let old_message_globals = message_globals.clone();
                message_globals = match status(
                    ctx.clone(),
                    message_globals,
                    statics.clone(),
                    update_avatar,
                )
                .await
                {
                    Ok((item, time)) => {
                        update_avatar = time;
                        item
                    }
                    Err(e) => {
                        log::error!("cant get new stats: {:#?}", e);
                        // return old if it cant find new details
                        old_message_globals.clone()
                    }
                };
                last_update.store(Utc::now().timestamp() / 60, atomic::Ordering::Relaxed);
                // wait 2 minutes before redo
                tokio::time::sleep(time::Duration::from_secs(60)).await;
            }
        });
    }
}

async fn status(
    ctx: Context,
    message_globals: message::Global,
    statics: message::Static,
    mut update_avatar: chrono::DateTime<Utc>,
) -> Result<(message::Global, chrono::DateTime<Utc>)> {
    let status =
        server_info::change_name(ctx.clone(), statics.clone(), &message_globals.game_id).await?;
    let image_loc = server_info::gen_img(status.clone(), statics.clone()).await?;

    // only allow updating once a minute to avoid spamming the avatar api
    if update_avatar.add(chrono::Duration::minutes(
        statics.mins_between_avatar_change.into(),
    )) <= chrono::Utc::now()
    {
        // change avatar
        let avatar = CreateAttachment::path(image_loc)
            .await
            .expect("Failed to read image");
        let mut user = ctx.cache.current_user().clone();

        let mut new_profile = EditProfile::new().avatar(&avatar);
        if &statics.set_banner_image[..] == "yes" {
            let banner = CreateAttachment::path("./map.jpg")
                .await
                .expect("Failed to read banner image");
            new_profile = new_profile.banner(&banner);
        }
        if let Err(e) = user.edit(ctx.clone(), new_profile).await {
            log::error!(
                "Failed to set new avatar: {:?}\n adding timeout before retrying",
                e
            );
            // add official avatar timeout if discord avatar timeout is reached
            update_avatar = chrono::Utc::now().add(chrono::Duration::minutes(5));
        } else {
            update_avatar = chrono::Utc::now();
        };
    }

    Ok((
        message::check(ctx, status.clone(), message_globals, statics).await?,
        update_avatar,
    ))
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    log::set_max_level(log::LevelFilter::Info);
    flexi_logger::Logger::try_with_str("warn,discord_bot=info")
        .unwrap_or_else(|e| panic!("Logger initialization failed with {:#?}", e))
        .start()?;

    // Login with a bot token from the environment
    let token = &env::var("token").expect("token wasn't given an argument!")[..];
    let intents = GatewayIntents::non_privileged();
    let mut client = Client::builder(token, intents)
        .event_handler(Handler)
        .await
        .expect("Error creating client");

    // start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        log::error!("Client error: {:?}", why);
    }
    Ok(())
}
