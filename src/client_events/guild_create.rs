use crate::{Bot, db::global::guild_data};
use sea_orm::ActiveValue::NotSet;
use sea_orm::{EntityTrait, Set, sea_query::OnConflict};
use serenity::model::guild::Guild;
use serenity::prelude::*;

pub async fn guild_create(bot: &Bot, _ctx: Context, guild: Guild, is_new: Option<bool>) {
    if is_new == Some(true) {
        return;
    }

    let data = guild_data::ActiveModel {
        guild_id: Set(guild.id.to_string()),
        name: Set(guild.clone().name),
        icon: Set(guild.clone().icon_url()),
        banner: Set(guild.clone().banner_url()),
        stat_exclusion_channels: Set(vec![]),
        join_log: NotSet,
        leave_log: NotSet,
    };

    let result = guild_data::Entity::insert(data.clone())
        .on_conflict(
            OnConflict::column(guild_data::COLUMN.guild_id)
                .update_columns([
                    guild_data::Column::Name,
                    guild_data::Column::Icon,
                    guild_data::Column::Banner,
                ])
                .to_owned(),
        )
        .exec(&bot.database)
        .await;

    if let Err(err) = result {
        println!("Error while updating database: {}", err);
    }
}
