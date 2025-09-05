use serenity::all::GuildId;
use serenity::async_trait;
use serenity::model::channel::Message;
use serenity::prelude::*;

use crate::create_client;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn message(&self, _ctx: Context, msg: Message) {
        if msg.author.bot { return () }
        let guild = match msg.guild_id {
            Some(guild) => guild,
            None => return ()
        };
        let client = match create_client().await {
            Ok(client) => client,
            Err(err) => {println!("{:?}", err); return ()}
        };
        match client.query_one("SELECT * FROM users.user_data WHERE user_id = $1;", &[&msg.author.id.to_string()]).await {
            Ok(_) => increment_user(client, msg, guild).await,
            Err(err) => {
                if err.code() != None {
                    println!("{:?}", err)
                } else {
                    create_user(client, msg, guild).await
                }
            }
        }
    }
}

async fn increment_user(client: tokio_postgres::Client, msg: Message, guild: GuildId) {
    match client.execute(
        "UPDATE users.user_data 
        SET total = total + 1, message_count = message_count + 1
        WHERE user_id = $1 AND guild_id = $2;", 
        &[&msg.author.id.to_string(), &guild.to_string()]
    ).await {
        Ok(_) => (),
        Err(err) => {println!("{:?}", err); return ()}
    }
}

async fn create_user(client: tokio_postgres::Client, msg: Message, guild: GuildId) {
    let user = msg.author;
    match client.execute(
        "INSERT INTO users.user_data 
        (user_id, guild_id, username, avatar, total, display_name, message_count, voice_time)
        VALUES($1, $2, $3, $4, 1, $5, 1, 0);", 
        &[&user.id.to_string(), &guild.to_string(), &user.name, &user.avatar_url(), &user.display_name()]
    ).await {
        Ok(_) => (),
        Err(err) => {println!("{:?}", err); return ()}
    }
}