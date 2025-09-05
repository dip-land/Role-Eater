use serde::{Deserialize, Serialize};
use serenity::prelude::*;
use tokio_postgres::{Client as PGClient, NoTls};

use dotenv::dotenv;
#[macro_use]
extern crate dotenv_codegen;

mod client_events;

#[tokio::main]
async fn main() {
    dotenv().ok();
    
    let intents = GatewayIntents::GUILDS | 
        GatewayIntents::GUILD_MESSAGES | 
        GatewayIntents::MESSAGE_CONTENT | 
        GatewayIntents::GUILD_MEMBERS | 
        GatewayIntents::GUILD_VOICE_STATES | 
        GatewayIntents::GUILD_PRESENCES;

    let mut client =
        Client::builder(&dotenv!("TOKEN"), intents)
        .event_handler(client_events::guild_create::Handler)
        .event_handler(client_events::guild_update::Handler)
        .event_handler(client_events::interaction_create::Handler)
        .event_handler(client_events::message::Handler)
        .event_handler(client_events::presence_update::Handler)
        .event_handler(client_events::user_update::Handler)
        .event_handler(client_events::voice_state_update::Handler)
        .await.expect("Err creating client");

    // Start listening for events by starting a single shard
    if let Err(why) = client.start().await {
        println!("Client error: {why:?}");
    }
}

pub async fn create_client() -> Result<PGClient, tokio_postgres::Error> {
    let (client, connection) = match tokio_postgres::connect(
        &format!("postgresql://{}:{}@localhost/role_eater", dotenv!("PG_USER"), dotenv!("PG_PASSWORD")), 
        NoTls
    ).await {
        Ok((client, connection)) => (client, connection),
        Err(err) => return Err(err)
    };

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("connection error: {}", e);
        }
    });

    Ok(client)
}


// All these structs are for later and will be for postgres
#[derive(Serialize, Deserialize, Debug)]
pub struct Guild {
    guild_id: String,
    name: String,
    icon: Option<String>,
    stat_exclusion_channels: Vec<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Role {
    role_id: String,
    guild_id: String,
    creator_id: Option<String>,
    name: Option<String>,
    color: String,
    is_admin: bool
}

#[derive(Serialize, Deserialize, Debug)]
pub struct User {
    user_id: String,
    guild_id: String,
    username: String,
    display_name: Option<String>,
    avatar: Option<String>,
    message_count: i64,
    voice_time: f64,
    total: f64,
    voice_channel_id: Option<String>,
    voice_channel_join_time: Option<String>
}

#[derive(Serialize, Deserialize, Debug)]
pub struct UserAsset {
    user_id: String,
    guild_id: String,
    asset: String
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActivityGameHistory {
    user_id: String,
    game_title: String,
    play_count: i64,
    time_played: f64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActivityMusicHistory {
    user_id: String,
    song_name: String,
    song_artist: String,
    play_count: i64,
    time_played: f64
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ActivityTimeHistory {
    user_id: String,
    date: String,
    game_time: Option<f64>,
    game_count: Option<i64>,
    music_time: Option<f64>,
    music_count: Option<i64>
}


#[derive(Serialize, Deserialize, Debug)]
pub struct VoiceMessageHistory {
    user_id: String,
    guild_id: String,
    date: String,
    message_count: i64,
    voice_time: f64
}