use serenity::async_trait;
use serenity::model::user::CurrentUser;
use serenity::prelude::*;

pub struct Handler;

#[async_trait]
impl EventHandler for Handler {
    async fn user_update(&self, ctx: Context, old: Option<CurrentUser>, new: CurrentUser) {
    }
}