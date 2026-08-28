use crate::{
    Bot,
    db::{get_user, increment_user},
};
use serenity::model::channel::Message;
use serenity::prelude::*;

pub async fn message(bot: &Bot, ctx: Context, msg: Message) {
    if msg.author.bot || msg.author.system {
        return;
    }
    let guild = match msg.guild_id {
        Some(guild) => guild,
        None => return,
    };

    let user = get_user(&bot.database, ctx.http.http(), &msg.author, &guild).await;

    if user.is_ok() {
        increment_user(&bot.database, &msg.author, &guild)
            .await
            .expect("Failed to increment user");
    }
}
