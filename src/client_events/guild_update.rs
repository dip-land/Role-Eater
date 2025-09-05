use serenity::async_trait;
use serenity::model::guild::{Guild, PartialGuild};
use serenity::prelude::*;

use crate::create_client;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_update(&self, _ctx: Context, _old_data: Option<Guild>, new_data: PartialGuild) {
        let client = match create_client().await {
            Ok(client) => client,
            Err(err) => {println!("{:?}", err); return ()}
        };
        match client.execute("UPDATE guilds.guild_data SET \"name\" = $2, icon = $3 WHERE guild_id = $1;", &[&new_data.id.to_string(), &new_data.name, &new_data.icon_url()]).await {
            Ok(_) => (),
            Err(err) => {println!("{:?}", err); return ()}
        }
    }
}