use serenity::async_trait;
use serenity::model::gateway::Presence;
use serenity::prelude::*;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn presence_update(&self, ctx: Context, new_data: Presence) {
    }
}