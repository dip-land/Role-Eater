use serenity::async_trait;
use serenity::model::guild::{Guild, PartialGuild};
use serenity::prelude::*;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn guild_update(&self, ctx: Context, old_data: Option<Guild>, new_data: PartialGuild) {
    }
}