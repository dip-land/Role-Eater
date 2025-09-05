use serenity::async_trait;
use serenity::model::guild::Guild;
use serenity::prelude::*;

use crate::create_client;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_create(&self, _ctx: Context, guild: Guild, is_new: Option<bool>) {
        if is_new.is_some_and(|v| v == true) { return }
        let client = match create_client().await {
            Ok(client) => client,
            Err(_) => return ()
        };

        let empty_string_vec: Vec<String> = Vec::new();
        match client.execute(
            "INSERT INTO guilds.guild_data (guild_id, \"name\", icon, stat_exclusion_channels) VALUES($1, $2, $3, $4);", 
            &[&guild.id.to_string(), &guild.name, &guild.icon_url(), &empty_string_vec]
        ).await {
            Ok(_) => (),
            Err(err) => {println!("{:?}", err); ()}
        }
    }
}