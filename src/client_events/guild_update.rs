use crate::{Bot, db::global::guild_data};
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serenity::model::guild::{Guild, PartialGuild};
use serenity::prelude::*;

pub async fn guild_update(
    bot: &Bot,
    _ctx: Context,
    _old_data: Option<Guild>,
    new_data: PartialGuild,
) {
    let guild: Option<guild_data::Model> = guild_data::Entity::find_by_id(new_data.id.to_string())
        .one(&bot.database)
        .await
        .unwrap();
    let mut guild: guild_data::ActiveModel = guild.unwrap().into();

    guild.name = Set(new_data.clone().name);
    guild.icon = Set(new_data.icon_url());

    if let Err(err) = guild.update(&bot.database).await {
        println!("{:?}", err);
    }
}
