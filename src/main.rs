use anyhow::Result;
use chrono::Utc;
use serenity::{
    client::{Client, Context, EventHandler},
    model::gateway::Ready,
    prelude::GatewayIntents,
};
use std::{
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
        let user = ctx.cache.current_user();
        log::info!("Logged in as {:#?}", user.name);

        let last_update = Arc::new(atomic::AtomicI64::new(0));
        let last_update_clone = Arc::clone(&last_update);

        let mut message_globals = message::Global {
            game_id: String::from(""),
            since_empty: false,
            previous_request: Vec::new(),
            since_player_trigger: 5,
            status: String::from(""),
        };

        let statics = message::Static {
            server_id: env::var("guid").unwrap_or_else(|_| "none".to_string()),
            game: env::var("game").unwrap_or_else(|_| "tunguska".to_string()),
            owner_id: env::var("ownerId").unwrap_or_else(|_| "none".to_string()),
            platform: env::var("platform").unwrap_or_else(|_| "pc".to_string()),
            fake_players: env::var("fakeplayers").unwrap_or_else(|_| "no".to_string()),
            server_name: env::var("name")
                .expect("name wasn't given an argument!")
                .replace('`', "#")
                .replace('*', "\\\""),
            lang: env::var("lang")
                .expect("lang wasn't given an argument!")
                .to_lowercase(),
            min_player_amount: env::var("minplayeramount")
                .expect("minplayeramount wasn't given an argument!")
                .parse::<i32>()
                .expect("I wasn't given an integer!"),
            amount_of_prev_request: env::var("prevrequestcount")
                .expect("prevrequestcount wasn't given an argument!")
                .parse::<i32>()
                .expect("I wasn't given an integer!"),
            message_channel: env::var("channel")
                .expect("channel wasn't given an argument!")
                .parse::<u64>()
                .expect("I wasn't given an integer!"),
            started_amount: env::var("startedamount")
                .expect("startedamount wasn't given an argument!")
                .parse::<i32>()
                .expect("I wasn't given an integer!"),
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

        // loop in seperate async
        tokio::spawn(async move {
            loop {
                let old_message_globals = message_globals.clone();
                message_globals = match status(ctx.clone(), message_globals, statics.clone()).await
                {
                    Ok(item) => item,
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
) -> Result<message::Global> {
    let status =
        server_info::change_name(ctx.clone(), statics.clone(), &message_globals.game_id).await?;
    let image_loc = server_info::gen_img(status.clone(), statics.clone()).await?;

    // change avatar
    let avatar = serenity::utils::read_image(image_loc).expect("Failed to read image");
    let mut user = ctx.cache.current_user();
    let _ = user.edit(&ctx, |p| p.avatar(Some(&avatar))).await;

    message::check(ctx, status.clone(), message_globals, statics).await
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
