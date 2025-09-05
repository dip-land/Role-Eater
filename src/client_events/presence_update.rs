use serenity::async_trait;
use serenity::model::gateway::Presence;
use serenity::prelude::*;

use std::time::SystemTime;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn presence_update(&self, _ctx: Context, new_data: Presence) {
        let timestamp = std::time::UNIX_EPOCH.elapsed().unwrap().as_millis();

        println!("{:?}", timestamp);

        for activity in new_data.activities {

        }
    }
}