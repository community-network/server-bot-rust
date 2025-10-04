use super::server_info;
use anyhow::Result;
use serenity::{
    builder::{CreateAttachment, CreateEmbed, CreateEmbedFooter, CreateMessage},
    client::Context,
    model::id::ChannelId,
};
use std::sync::LazyLock;

#[derive(Clone, Debug)]
pub struct Global {
    pub server_name: String,
    pub game_id: String,
    pub since_empty: bool,
    pub previous_request: Vec<i32>,
    pub since_player_trigger: i32,
}

static DEFAULT_G: LazyLock<Global> = LazyLock::new(|| Global {
    server_name: "".to_string(),
    game_id: "".to_string(),
    since_empty: false,
    previous_request: Vec::new(),
    since_player_trigger: 5,
});

impl<'a> Default for &'a Global {
    fn default() -> &'a Global {
        &DEFAULT_G
    }
}

impl Default for Global {
    fn default() -> Global {
        DEFAULT_G.clone()
    }
}

#[derive(Clone, Debug)]
pub struct Static {
    pub set_banner_image: String,
    pub lang: String,
    pub mins_between_avatar_change: i32,
    pub servers: Vec<Server>,
    pub min_player_amount: i32,
    pub amount_of_prev_request: i32,
    pub message_channel: u64,
    pub started_amount: i32,
    pub include_spectators: String,
    pub game: String,
    pub platform: String,
    pub multiple_msg: String,
}

#[derive(Clone, Debug)]
pub struct Server {
    pub server_name: String,
    pub server_id: String,
    pub owner_id: String,
    pub fake_players: String,
}

pub async fn check(
    ctx: Context,
    status: server_info::ServerInfo,
    mut globals: Global,
    statics: &Static,
) -> Result<Global> {
    if statics.message_channel != 40 {
        // in_spectator
        let server_info = format!(
            "{}/{}{}{} - {}",
            status.detailed.current_players,
            status.detailed.max_players,
            match status.detailed.in_que.unwrap_or(0) > 0 {
                true => format!(" [{}]", status.detailed.in_que.unwrap_or(0)),
                false => "".to_string(),
            },
            match &statics.include_spectators[..] == "yes" {
                true => format!("({})", status.detailed.in_spectator.unwrap_or(0)),
                false => "".to_string(),
            },
            status.detailed.server_map
        );

        let mut image_url = "info_image.jpg";
        if status.detailed.server_name.contains("AMG") {
            image_url = "only_favorites_image.jpg";
        }

        let mut test = false;
        for request in globals.clone().previous_request.iter() {
            if request - status.detailed.current_players >= statics.min_player_amount
                && globals.since_player_trigger > (statics.amount_of_prev_request * 4)
            {
                // check last few requests for changes
                globals.since_player_trigger = 0;
                test = true;
                break;
            }
        }

        if test {
            // if prev test true, send message
            send(
                ctx.clone(),
                statics.clone(),
                image_url,
                status.clone(),
                "I'm low on players! Join me now!",
                &format!("Perfect time to join without queue!\n{}", server_info)[..],
            )
            .await?;
        } else {
            // if none worked
            globals.since_player_trigger += 1;
        }

        if status.detailed.current_players <= 5 {
            // counter since empty
            globals.since_empty = true
        }

        if globals.since_empty && status.detailed.current_players >= statics.started_amount {
            // run if 1 hour after starting and playercount is good
            if status.detailed.server_name.contains("AMG") {
                send(
                    ctx.clone(),
                    statics.clone(),
                    image_url,
                    status.clone(),
                    "I'm up and running!",
                    &format!("Feeling good :slight_smile:\n{}", server_info)[..],
                )
                .await?;
            }
            globals.since_empty = false;
        }

        if status.detailed.current_players >= statics.min_player_amount
            && globals.previous_request.len() as i32 >= (statics.amount_of_prev_request * 2)
        {
            // if current is above or at 20 players and runs for at least a few mins
            if statics.min_player_amount > *globals.previous_request.iter().max().unwrap() {
                // # if the past messages are below 20
                send(
                    ctx.clone(),
                    statics.clone(),
                    image_url,
                    status.clone(),
                    "Pre-round is over!",
                    &format!(
                        "No more waiting. If you join now you can instantly play.\n{}",
                        server_info
                    )[..],
                )
                .await?;
            }
        }

        if globals.previous_request.len() as i32 >= (statics.amount_of_prev_request * 2) {
            // if it has run more than 4 times
            globals.previous_request.remove(0); // remove first item
        }
        globals
            .previous_request
            .push(status.detailed.current_players); // add current in back
    };

    globals.game_id = status.shared_info.game_id;
    Ok(globals)
}

pub async fn send(
    ctx: Context,
    statics: Static,
    image_url: &str,
    status: server_info::ServerInfo,
    title: &str,
    description: &str,
) -> Result<serenity::model::channel::Message, serenity::Error> {
    let paths = CreateAttachment::path(image_url).await?;
    let games = std::collections::HashMap::from([
        ("tunguska", "bf1"),
        ("casablanca", "bfv"),
        ("kingston", "bf2042"),
    ]);
    let mut gather_type = "gameid";
    if statics.game == "kingston" {
        gather_type = "serverid";
    } else if status.shared_info.clone().game_id.contains(':') {
        gather_type = "serverip";
    }
    let server_link = format!(
        "https://gametools.network/servers/{}/{}/{}/{}",
        games.get(&statics.game[..]).unwrap_or(&&statics.game[..]),
        gather_type,
        status.shared_info.clone().game_id,
        statics.platform
    );
    let footer = CreateEmbedFooter::new(format!("player threshold set to {} players, checks difference of previous {} minutes and in-between",
    statics.min_player_amount, statics.amount_of_prev_request*2));
    let embed = CreateEmbed::new()
        .url(server_link)
        .title(title)
        .description(description)
        .footer(footer);
    let builder = match status.detailed.server_name.contains("AMG") {
        true => CreateMessage::new().embed(embed.image(format!("attachment://{}", image_url))),
        false => CreateMessage::new().embed(embed.thumbnail(format!("attachment://{}", image_url))),
    };
    ChannelId::new(statics.message_channel)
        .send_files(&ctx.http, [paths], builder)
        .await
}
