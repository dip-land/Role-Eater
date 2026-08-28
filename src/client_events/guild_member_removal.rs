use crate::{Bot, db::global::guild_data};
use chrono::Local;
use sea_orm::EntityTrait;
use serenity::{
    builder::{CreateEmbed, CreateEmbedAuthor, CreateMessage},
    model::{
        guild::Member,
        id::{ChannelId, GuildId},
        user::User,
    },
    prelude::*,
};

pub async fn guild_member_removal(
    bot: &Bot,
    ctx: Context,
    guild_id: GuildId,
    user: User,
    _member_data_if_available: Option<Member>,
) {
    let guild: Option<guild_data::Model> = guild_data::Entity::find_by_id(guild_id.to_string())
        .one(&bot.database)
        .await
        .unwrap();

    if let Some(guild) = guild {
        if guild.leave_log.is_none() {
            return;
        }

        let channel = ChannelId::new(guild.leave_log.unwrap().parse().unwrap());

        let author =
            CreateEmbedAuthor::new(format!("{} ({}) Left", user.name, user.id.to_string()))
                .icon_url(user.avatar_url().unwrap());
        let embed = CreateEmbed::new().author(author).description(format!(
            "Created: <t:{:?}:f> (<t:{:?}:R>) \nLeft: <t:{:?}:f> (<t:{:?}:R>)",
            user.created_at().timestamp(),
            user.created_at().timestamp(),
            Local::now().timestamp(),
            Local::now().timestamp()
        ));
        let response = CreateMessage::new().add_embed(embed);

        let result = channel.send_message(&ctx.http, response).await;
        if let Err(err) = result {
            println!("Error while sending leave message: {}", err);
        }
    }
}
