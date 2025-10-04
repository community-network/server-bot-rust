use crate::message::Global;

use super::message;
use ab_glyph::{FontRef, PxScale};
use anyhow::Result;
use image::{ImageReader, Rgba};
use imageproc::drawing::draw_text_mut;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::{client::Context, gateway::ActivityData};
use std::io::Cursor;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MainInfo {
    #[serde(rename = "playerAmount")]
    pub current_players: i32,
    #[serde(rename = "maxPlayers")]
    pub max_players: i32,
    #[serde(rename = "inQue")]
    pub in_que: Option<i32>,
    #[serde(rename = "inSpectator")]
    pub in_spectator: Option<i32>,
    #[serde(rename = "smallMode")]
    pub small_mode: String,
    #[serde(rename = "currentMap")]
    pub server_map: Option<String>,
    pub map: Option<String>,
    #[serde(rename = "url")]
    pub map_url: Option<String>,
    #[serde(rename = "mapImage")]
    pub map_image: Option<String>,
    #[serde(rename = "mode")]
    pub map_mode: Option<String>,
    #[serde(rename = "prefix")]
    pub server_name: Option<String>,
    pub server: Option<String>,
    pub region: Option<String>,
    #[serde(rename = "gameId")]
    pub game_id: Option<String>,
    #[serde(rename = "ownerId")]
    pub owner_id: Option<String>,
    #[serde(rename = "serverId")]
    pub server_id: Option<String>,
    pub ip: Option<String>,
    pub port: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DetailedInfo {
    #[serde(rename = "playerAmount")]
    pub current_players: i32,
    #[serde(rename = "maxPlayerAmount")]
    pub max_players: i32,
    #[serde(rename = "inQueue")]
    pub in_que: Option<i32>,
    #[serde(rename = "inSpectator")]
    pub in_spectator: Option<i32>,
    #[serde(rename = "smallmode")]
    pub small_mode: String,
    #[serde(rename = "prefix")]
    pub server_name: String,

    #[serde(rename = "currentMap")]
    pub server_map: String,
    #[serde(rename = "currentMapImage")]
    pub map_url: String,
    #[serde(rename = "mode")]
    pub map_mode: String,
    pub region: String,

    pub favorites: String,
    #[serde(rename = "noBotsPlayerAmount")]
    pub fake_players: Option<i32>,
}

#[derive(Debug, Clone)]
pub struct ServerInfo {
    pub shared_info: Global,
    pub detailed: DetailedInfo,
}

async fn request_list(
    server: &message::Server,
    statics: &message::Static,
    game: &str,
    client: &reqwest::Client,
) -> Result<serde_json::Value> {
    let mut url =
        Url::parse(&format!("https://api.gametools.network/{}/servers/", game)[..]).unwrap();
    url.query_pairs_mut()
        .append_pair("name", &server.server_name[..])
        .append_pair("lang", &statics.lang[..])
        .append_pair("limit", "10");

    Ok(client
        .get(url)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?)
}

async fn request_detailed(
    statics: &message::Static,
    game_id: &str,
    game: &str,
    client: &reqwest::Client,
) -> Result<serde_json::Value> {
    let mut url =
        Url::parse(&format!("https://api.gametools.network/{}/detailedserver/", game)[..]).unwrap();
    url.query_pairs_mut()
        .append_pair("gameid", game_id)
        .append_pair("lang", &statics.lang[..]);

    Ok(client
        .get(url)
        .send()
        .await?
        .json::<serde_json::Value>()
        .await?)
}

async fn get(
    statics: message::Static,
    server: message::Server,
    shared_info: &Global,
) -> Result<ServerInfo> {
    let game;
    if &statics.game[..] == "tunguska" {
        game = "bf1"
    } else if &statics.game[..] == "casablanca" {
        game = "bfv"
    } else if &statics.game[..] == "kingston" {
        game = "bf2042"
    } else {
        game = &statics.game[..]
    }

    let client = reqwest::Client::new();
    // try twice first
    let mut response = request_list(&server, &statics, game, &client).await?;
    if response.get("errors").is_some() {
        response = request_list(&server, &statics, game, &client).await?;
    }

    let mut info = json!(null);

    // get via ownerid if newer than bf1
    if &server.owner_id[..] != "none"
        && (&statics.game[..] == "casablanca" || &statics.game[..] == "kingston")
    {
        // fail on error
        let servers = match response.get("servers") {
            Some(result) => result.as_array().unwrap(),
            None => anyhow::bail!("Failed to get serverlist from main api"),
        };
        // use ownerid to select server
        for (i, cur) in servers.iter().enumerate() {
            if serde_json::from_value::<MainInfo>(cur.to_owned())?
                .owner_id
                .unwrap_or_default()
                == server.owner_id
            {
                info = response["servers"][i].to_owned();
                break;
            }
        }
    // try with guid (which should be static)
    } else if &server.server_id[..] != "none" {
        // fail on error
        let servers = match response.get("servers") {
            Some(result) => result.as_array().unwrap(),
            None => anyhow::bail!("Failed to get serverlist from main api"),
        };
        // use ownerid to select server
        for (i, cur) in servers.iter().enumerate() {
            if serde_json::from_value::<MainInfo>(cur.to_owned())?
                .server_id
                .unwrap_or_default()
                == server.server_id
            {
                info = response["servers"][i].to_owned();
                break;
            }
        }
    } else {
        // get first server or null (for game_id)
        if response.get("errors").is_none() {
            info = response["servers"][0].to_owned();
        }
    }

    // update game_id if it can be gathered
    let mut game_id = shared_info.game_id.to_string();
    if !info.is_null() {
        let server_info = serde_json::from_value::<MainInfo>(info.clone())?;
        if game == "bf2042" {
            game_id = server_info.server_id.unwrap_or_default();
        } else if server_info.game_id.is_none()
            && server_info.ip.is_some()
            && server_info.port.is_some()
        {
            game_id = format!(
                "{}:{}",
                server_info.ip.unwrap_or_default(),
                server_info.port.unwrap_or_default()
            );
        } else {
            game_id = server_info.game_id.unwrap_or_default();
        }
    }

    // get detailed via old or new game_id
    let detailed = match &statics.game[..] {
        "tunguska" | "bf4" => {
            let mut detailed_response = request_detailed(&statics, &game_id, game, &client).await?;
            if detailed_response.get("errors").is_some() {
                detailed_response = request_detailed(&statics, &game_id, game, &client).await?;
            }

            let mut detailed = serde_json::from_value::<DetailedInfo>(detailed_response)?;

            if &statics.game[..] == "bf4" && &server.fake_players[..] == "yes" {
                detailed.current_players = detailed.fake_players.unwrap_or_default();
            }
            detailed
        }
        _ => {
            let payload = serde_json::from_value::<MainInfo>(info)?;
            DetailedInfo {
                current_players: payload.current_players,
                max_players: payload.max_players,
                in_spectator: payload.in_spectator,
                in_que: payload.in_que,
                small_mode: payload.small_mode,
                server_name: payload
                    .server_name
                    .unwrap_or(payload.server.unwrap_or_default()),
                server_map: match payload.server_map {
                    Some(map_name) => map_name,
                    None => payload.map.unwrap_or_default(),
                },
                map_url: match payload.map_url {
                    Some(map_url) => map_url,
                    None => payload.map_image.unwrap_or_default(),
                },
                map_mode: payload.map_mode.unwrap_or_default(),
                region: match payload.region {
                    Some(region) => region,
                    None => "".into(),
                },
                favorites: "0".to_string(),
                fake_players: Some(0),
            }
        }
    };

    // game_id is saved if server cant be found with search
    Ok(ServerInfo {
        shared_info: Global {
            server_name: server.server_name,
            game_id: game_id,
            since_empty: shared_info.since_empty,
            previous_request: shared_info.previous_request.clone(),
            since_player_trigger: shared_info.since_player_trigger,
        },
        detailed,
    })
}

pub async fn change_name(
    ctx: Context,
    statics: message::Static,
    game_ids: &Vec<Global>,
) -> Result<Vec<ServerInfo>> {
    let mut results = Vec::new();
    for (i, server) in statics.servers.iter().enumerate() {
        match get(
            statics.clone(),
            server.clone(),
            game_ids.get(i).unwrap_or_default(),
        )
        .await
        {
            Ok(status) => results.push(status.clone()),
            Err(e) => {
                if statics.servers.len() > 1 {
                    println!(
                        "Failed to get new serverinfo for server '{}': {:#?}",
                        server.server_name, e
                    )
                } else {
                    let server_info = "¯\\_(ツ)_/¯ server not found";
                    ctx.set_activity(Some(ActivityData::playing(server_info)));

                    anyhow::bail!(format!("Failed to get new serverinfo: {:#?}", e))
                }
            }
        };
    }

    if statics.servers.len() > 1 {
        let mut total = DetailedInfo {
            current_players: 0,
            max_players: 0,
            in_que: Some(0),
            in_spectator: Some(0),
            small_mode: "".to_string(),
            server_name: "".to_string(),
            server_map: "".to_string(),
            map_url: "".to_string(),
            map_mode: "".to_string(),
            region: "".to_string(),
            favorites: "".to_string(),
            fake_players: Some(0),
        };
        for server in results.clone() {
            total.current_players += server.detailed.current_players;
            total.max_players += server.detailed.max_players;
            total.in_que = match total.in_que {
                Some(x) => Some(x + server.detailed.in_que.unwrap_or_default()),
                None => Some(server.detailed.in_que.unwrap_or_default()),
            };
            total.in_spectator = match total.in_spectator {
                Some(x) => Some(x + server.detailed.in_spectator.unwrap_or_default()),
                None => Some(server.detailed.in_spectator.unwrap_or_default()),
            };
            total.fake_players = match total.fake_players {
                Some(x) => Some(x + server.detailed.fake_players.unwrap_or_default()),
                None => Some(server.detailed.fake_players.unwrap_or_default()),
            };
        }
        let server_info = format!(
            "{}/{}{}{} {}",
            total.current_players,
            total.max_players,
            match total.in_que.unwrap_or(0) > 0 {
                true => format!(" [{}]", total.in_que.unwrap_or(0)),
                false => "".to_string(),
            },
            match &statics.include_spectators[..] == "yes" {
                true => format!(" ({})", total.in_spectator.unwrap_or(0)),
                false => "".to_string(),
            },
            statics.multiple_msg
        );

        // change game activity
        ctx.set_activity(Some(ActivityData::playing(server_info)));
        return Ok(results.clone());
    } else {
        let status = results[0].clone();
        let server_info = format!(
            "{}/{}{}{} - {}",
            status.detailed.current_players,
            status.detailed.max_players,
            match status.detailed.in_que.unwrap_or(0) > 0 {
                true => format!(" [{}]", status.detailed.in_que.unwrap_or(0)),
                false => "".to_string(),
            },
            match &statics.include_spectators[..] == "yes" {
                true => format!(" ({})", status.detailed.in_spectator.unwrap_or(0)),
                false => "".to_string(),
            },
            status.detailed.server_map
        );

        // change game activity
        ctx.set_activity(Some(ActivityData::playing(server_info)));
        return Ok(vec![status.clone()]);
    }
}

pub async fn gen_img(status: ServerInfo, statics: message::Static) -> Result<String> {
    let client = reqwest::Client::new();
    let img = client
        .get(status.detailed.map_url.replace(
            "[BB_PREFIX]",
            "https://eaassets-a.akamaihd.net/battlelog/battlebinary",
        ))
        .send()
        .await?
        .bytes()
        .await?;
    let mut img2 = ImageReader::new(Cursor::new(img))
        .with_guessed_format()?
        .decode()?;

    img2.save("./map.jpg")?;
    img2 = img2.brighten(-25);

    let font: FontRef = if &statics.game[..] == "kingston" || &statics.game[..] == "bf2042" {
        FontRef::try_from_slice(include_bytes!("BF_Modernista-Regular.ttf") as &[u8]).unwrap()
    } else {
        FontRef::try_from_slice(include_bytes!("Futura.ttf") as &[u8]).unwrap()
    };

    let small_font = FontRef::try_from_slice(include_bytes!("DejaVuSans.ttf") as &[u8]).unwrap();

    let img_size = PxScale {
        x: img2.width() as f32,
        y: img2.height() as f32,
    };
    let mut orig_img2 = img2.clone();

    // only smallmode
    let scale = PxScale {
        x: (img2.width() / 3) as f32,
        y: (img2.height() as f32 / 1.9),
    };
    let mut middle = 3.5;
    if status.detailed.map_mode == "TugOfWar" {
        middle = 3.0;
    } else if &statics.game[..] == "kingston" || &statics.game[..] == "bf2042" {
        middle = 3.15;
    }

    draw_text_mut(
        &mut img2,
        Rgba([255u8, 255u8, 255u8, 255u8]),
        (img_size.x / middle) as i32,
        (img_size.y / 4.8) as i32,
        scale,
        &font,
        &status.detailed.small_mode[..],
    );
    img2.save("./map_mode.jpg")?;

    // with favorites except bf5
    let small_scale = PxScale {
        x: (img2.width() / 9) as f32,
        y: (img2.height() / 6) as f32,
    };
    if &statics.game[..] == "tunguska" || &statics.game[..] == "bf4" {
        draw_text_mut(
            &mut img2,
            Rgba([255u8, 255u8, 255u8, 255u8]),
            (img_size.x / 3.5) as i32,
            (img_size.y / 1.5) as i32,
            small_scale,
            &small_font,
            &format!("{}{}", "\u{2605}", status.detailed.favorites)[..],
        );
    }
    img2.save("./info_image.jpg")?;

    // only favorites except bf5
    let fav_scale = PxScale {
        x: (img2.width() / 7) as f32,
        y: (img2.height() as f32) / 4.5,
    };
    if &statics.game[..] == "tunguska" || &statics.game[..] == "bf4" {
        draw_text_mut(
            &mut orig_img2,
            Rgba([255u8, 255u8, 255u8, 255u8]),
            (img_size.x / 4.0) as i32,
            (img_size.y / 2.5) as i32,
            fav_scale,
            &small_font,
            &format!("{}{}", "\u{2605}", status.detailed.favorites)[..],
        );
    }
    orig_img2.save("./only_favorites_image.jpg")?;

    // get image based on name
    if status.detailed.server_name.contains("AMG") {
        return Ok(String::from("./only_favorites_image.jpg"));
    }
    Ok(String::from("./info_image.jpg"))
}
