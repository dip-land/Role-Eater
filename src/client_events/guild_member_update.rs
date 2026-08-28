use crate::Bot;
use crate::db::get_user;
use crate::db::users::user_data;
use sea_orm::{EntityTrait, IntoActiveModel, Set};
use serenity::all::GuildMemberUpdateEvent;
use serenity::model::guild::Member;
use serenity::prelude::*;

pub async fn guild_member_update(
    bot: &Bot,
    ctx: Context,
    _old_if_available: Option<Member>,
    new: Option<Member>,
    _event: GuildMemberUpdateEvent,
) {
    let member = new.unwrap();
    let user = get_user(
        &bot.database,
        ctx.http.http(),
        &member.user,
        &member.guild_id,
    )
    .await;

    let user = match user {
        Ok(user) => user,
        Err(error) => return println!("Error creating user: {error:?}"),
    };

    let mut user = user.into_active_model();
    user.username = Set(member.user.name.clone());
    user.display_name = Set(Some(member.display_name().to_string()));
    user.global_name = Set(member.user.global_name.clone());
    user.nickname = Set(member.nick.clone());
    user.avatar = Set(member.avatar_url());
    user.banner = Set(member.banner_url());

    let result = user_data::Entity::update(user.clone())
        .exec(&bot.database)
        .await;

    if let Err(err) = result {
        println!("Error while updating database: {}", err);
    }
}
