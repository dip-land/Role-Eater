use crate::Bot;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

pub async fn ready(bot: &Bot, _ctx: Context, data_about_bot: Ready) {
    println!("v.{}\n{:?}", bot.version, data_about_bot);
}
