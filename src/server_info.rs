use super::message;
use anyhow::Result;
use image::{io::Reader as ImageReader, Rgba};
use imageproc::drawing::draw_text_mut;
use reqwest::Url;
use rusttype::{Font, Scale};
use serde::{Deserialize, Serialize};
use serde_json::json;
use serenity::{client::Context, model::gateway::Activity};
use std::io::Cursor;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MainInfo {
    #[serde(rename = "playerAmount")]
    pub current_players: i32,
    #[serde(rename = "maxPlayers")]
    pub max_players: i32,
    #[serde(rename = "inQue")]
    pub in_que: Option<i32>,
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
    pub server_name: String,
    pub region: Option<String>,
    #[serde(rename = "gameId")]
    pub game_id: Option<String>,
    #[serde(rename = "ownerId")]
    pub owner_id: Option<String>,
    #[serde(rename = "serverId")]
    pub server_id: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DetailedInfo {
    #[serde(rename = "playerAmount")]
    pub current_players: i32,
    #[serde(rename = "maxPlayerAmount")]
    pub max_players: i32,
    #[serde(rename = "inQueue")]
    pub in_que: Option<i32>,
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
    pub game_id: Option<String>,
    pub detailed: DetailedInfo,
}

async fn request_list(
    statics: &message::Static,
    game: &str,
    client: &reqwest::Client,
) -> Result<serde_json::Value> {
    let mut url =
        Url::parse(&format!("https://api.gametools.network/{}/servers/", game)[..]).unwrap();
    url.query_pairs_mut()
        .append_pair("name", &statics.server_name[..])
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

async fn get(statics: message::Static, game_id: &String) -> Result<ServerInfo> {
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
    let mut response = request_list(&statics, game, &client).await?;
    if response.get("errors").is_some() {
        response = request_list(&statics, game, &client).await?;
    }

    let mut info = json!(null);

    // get via ownerid if newer than bf1
    if &statics.owner_id[..] != "none"
        && (&statics.game[..] != "tunguska" && &statics.game[..] != "bf4")
    {
        // fail on error
        let servers = match response.get("servers") {
            Some(result) => result.as_array().unwrap(),
            None => anyhow::bail!("Failed to get serverlist from main api"),
        };
        // use ownerid to select server
        for (i, server) in servers.iter().enumerate() {
            if serde_json::from_value::<MainInfo>(server.to_owned())?
                .owner_id
                .unwrap_or_default()
                == statics.owner_id
            {
                info = response["servers"][i].to_owned();
                break;
            }
        }
    // try with guid (which should be static)
    } else if &statics.server_id[..] != "none" {
        // fail on error
        let servers = match response.get("servers") {
            Some(result) => result.as_array().unwrap(),
            None => anyhow::bail!("Failed to get serverlist from main api"),
        };
        // use ownerid to select server
        for (i, server) in servers.iter().enumerate() {
            if serde_json::from_value::<MainInfo>(server.to_owned())?
                .server_id
                .unwrap_or_default()
                == statics.server_id
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
    let mut game_id = game_id.to_string();
    if !info.is_null() {
        game_id = serde_json::from_value::<MainInfo>(info.clone())?
            .game_id
            .unwrap_or_default();
    }

    // get detailed via old or new game_id
    let detailed = match &statics.game[..] {
        "tunguska" | "bf4" => {
            let mut detailed_response = request_detailed(&statics, &game_id, game, &client).await?;
            if detailed_response.get("errors").is_some() {
                detailed_response = request_detailed(&statics, &game_id, game, &client).await?;
            }

            let mut detailed = serde_json::from_value::<DetailedInfo>(detailed_response)?;

            if &statics.game[..] == "bf4" && &statics.fake_players[..] == "yes" {
                detailed.current_players = detailed.fake_players.unwrap_or_default();
            }
            detailed
        }
        _ => {
            let payload = serde_json::from_value::<MainInfo>(info)?;
            DetailedInfo {
                current_players: payload.current_players,
                max_players: payload.max_players,
                in_que: payload.in_que,
                small_mode: payload.small_mode,
                server_name: payload.server_name,
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
        game_id: Some(game_id),
        detailed,
    })
}

pub async fn change_name(
    ctx: Context,
    statics: message::Static,
    game_id: &String,
) -> Result<ServerInfo> {
    let status = match get(statics, game_id).await {
        Ok(status) => {
            let mut server_info = format!(
                "{}/{} [{}] - {}",
                status.detailed.current_players,
                status.detailed.max_players,
                status.detailed.in_que.unwrap_or(0),
                status.detailed.server_map,
            );
            if status.detailed.in_que.unwrap_or(0) == 0 {
                server_info = format!(
                    "{}/{} - {}",
                    status.detailed.current_players,
                    status.detailed.max_players,
                    status.detailed.server_map,
                );
            }
            // change game activity
            ctx.set_activity(Activity::playing(server_info)).await;

            status
        }
        Err(e) => {
            let server_info = "¯\\_(ツ)_/¯ server not found";
            ctx.set_activity(Activity::playing(server_info)).await;

            anyhow::bail!(format!("Failed to get new serverinfo: {}", e))
        }
    };

    Ok(status)
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
        .decode()?
        .brighten(-25);

    let font: Font = if &statics.game[..] == "kingston" || &statics.game[..] == "bf2042" {
        let font_name = Vec::from(include_bytes!("BF_Modernista-Regular.ttf") as &[u8]);
        Font::try_from_vec(font_name).unwrap()
    } else {
        let font_name = Vec::from(include_bytes!("Futura.ttf") as &[u8]);
        Font::try_from_vec(font_name).unwrap()
    };

    let small_font = Vec::from(include_bytes!("DejaVuSans.ttf") as &[u8]);
    let small_font = Font::try_from_vec(small_font).unwrap();

    let img_size = Scale {
        x: img2.width() as f32,
        y: img2.height() as f32,
    };
    let mut orig_img2 = img2.clone();

    // only smallmode
    let scale = Scale {
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
    let small_scale = Scale {
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
    let fav_scale = Scale {
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
