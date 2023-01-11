use serenity::{client::Context, model::id::ChannelId};
use anyhow::Result;
use super::server_info;

#[derive(Clone, Debug)]
pub struct Global {
    pub game_id: String,
    pub since_empty: bool,
    pub previous_request: Vec<i32>,
    pub since_player_trigger: i32,
    pub status: String
}

#[derive(Clone, Debug)]
pub struct Static {
    pub server_id: String,
    pub game: String,
    pub owner_id: String,
    pub fake_players: String,
    pub server_name: String,
    pub lang: String,
    pub min_player_amount: i32,
    pub amount_of_prev_request: i32,
    pub message_channel: u64,
    pub started_amount: i32
}

pub async fn check(ctx: Context, status: server_info::ServerInfo, mut globals: Global, statics: Static) -> Result<Global> {
    if statics.message_channel != 40 {
        let mut server_info = format!("{}/{} [{}] - {}", status.detailed.current_players, status.detailed.max_players,
            status.detailed.in_que.unwrap_or(0), status.detailed.server_map);
        if status.detailed.in_que.unwrap_or(0) == 0 {
            server_info = format!("{}/{} - {}", status.detailed.current_players, status.detailed.max_players,
                status.detailed.server_map);
        }

        let mut image_url = "info_image.jpg";
        if status.detailed.server_name.contains("AMG") {
            image_url = "only_favorites_image.jpg";
        }

        let mut test = false;
        for request in globals.clone().previous_request.iter() {
            if request - status.detailed.current_players >= statics.min_player_amount &&
                globals.since_player_trigger > (statics.amount_of_prev_request*4) { // check last few requests for changes
                globals.since_player_trigger = 0;
                test = true;
                break;
            }
        };

        if test { // if prev test true, send message
            let _ = send(ctx.clone(), statics.clone(), image_url, status.clone(), "I'm low on players! Join me now!", 
            &format!("Perfect time to join without queue!\n{}", server_info)[..]).await;
        } else { // if none worked
            globals.since_player_trigger += 1;
        } 

        if status.detailed.current_players <= 5 { // counter since empty
            globals.since_empty = true
        }

        if globals.since_empty && status.detailed.current_players >= statics.started_amount { // run if 1 hour after starting and playercount is good
            if status.detailed.server_name.contains("AMG") {
                let _ = send(ctx.clone(), statics.clone(), image_url, status.clone(), "I'm up and running!", 
                &format!("Feeling good :slight_smile:\n{}", server_info)[..]).await;
            }
            globals.since_empty = false;
        }

        if status.detailed.current_players >= statics.min_player_amount && globals.previous_request.len() as i32
            >= (statics.amount_of_prev_request*2) { // if current is above or at 20 players and runs for at least a few mins
                if statics.min_player_amount > *globals.previous_request.iter().max().unwrap() { // # if the past messages are below 20
                    let _ = send(ctx.clone(), statics.clone(), image_url, status.clone(), "Pre-round is over!", 
                    &format!("No more waiting. If you join now you can instantly play.\n{}", server_info)[..]).await;
                }
        }
        
        if globals.previous_request.len() as i32 >= (statics.amount_of_prev_request*2) { // if it has run more than 4 times
            globals.previous_request.remove(0); // remove first item
        }
        globals.previous_request.push(status.detailed.current_players); // add current in back
    };

    globals.game_id = status.game_id.unwrap_or_default();
    Ok(globals)
}

pub async fn send(ctx: Context, statics: Static, image_url: &str, status: server_info::ServerInfo, title: &str,
        description: &str) -> Result<serenity::model::channel::Message, serenity::Error> {
    let paths = vec![image_url];
    ChannelId(statics.message_channel).send_files(&ctx.http, paths, |m| {    
        m.embed(|e| {
            e.url(format!("https://gametools.network/servers/bf1/gameid/{}/pc", status.game_id.unwrap_or_default()));
            e.title(title);
            e.description(description);
            e.footer(|f| {
                f.text(format!("player threshold set to {} players, checks difference of previous {} minutes and in-between",
                    statics.min_player_amount, statics.amount_of_prev_request*2));
                
                f
            });
            if status.detailed.server_name.contains("AMG") {
                e.image(format!("attachment://{}", image_url));
            } else {
                e.thumbnail(format!("attachment://{}", image_url));
            }

            e
        });
    
        m
    }).await
}