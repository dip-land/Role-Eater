use chrono::{DateTime, Local};
use sea_orm::prelude::DateTimeWithTimeZone;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, Set,
};
use serenity::all::{GuildId, User as SerenityUser};
use serenity::http::Http;

pub mod global;
pub mod users;

pub async fn increment_user(
    db: &DatabaseConnection,
    user: &SerenityUser,
    guild: &GuildId,
) -> Result<users::user_data::Model, DbErr> {
    let update_user: Option<users::user_data::Model> = users::user_data::Entity::find()
        .filter(users::user_data::Column::UserId.eq(user.id.to_string()))
        .filter(users::user_data::Column::GuildId.eq(guild.to_string()))
        .one(db)
        .await?;
    let mut update_user: users::user_data::ActiveModel = update_user.unwrap().into();

    update_user.total = Set(update_user.total.unwrap() + 1.0);
    update_user.message_count = Set(update_user.message_count.unwrap() + 1);

    update_user.update(db).await
}

pub async fn get_user(
    db: &DatabaseConnection,
    http: &Http,
    user: &SerenityUser,
    guild: &GuildId,
) -> Result<users::user_data::Model, DbErr> {
    let mut user_data = users::user_data::Entity::find()
        .filter(users::user_data::Column::GuildId.eq(&guild.to_string()))
        .filter(users::user_data::Column::UserId.eq(&user.id.to_string()))
        .one(db)
        .await?;

    if user_data.is_none() {
        user_data = Some(create_user(db, http, user, guild).await?);
    }

    Ok(user_data.unwrap())
}

pub async fn create_user(
    db: &DatabaseConnection,
    http: &Http,
    user: &SerenityUser,
    guild: &GuildId,
) -> Result<users::user_data::Model, DbErr> {
    let member = guild.member(http, &user.id).await;

    let user_creation_date: DateTime<Local> =
        DateTime::from_timestamp_millis(user.clone().created_at().timestamp_millis())
            .unwrap()
            .into();
    let mut user_join_date: DateTime<Local> = DateTime::from_timestamp_millis(0).unwrap().into();

    let mut nickname: Option<String> = None;

    if let Ok(member) = member {
        user_join_date =
            DateTime::from_timestamp_millis(member.clone().joined_at.unwrap().timestamp_millis())
                .unwrap()
                .into();

        nickname = member.nick;
    }

    let user = users::user_data::ActiveModel {
        guild_id: Set(guild.to_string()),
        user_id: Set(user.id.to_string()),
        username: Set(user.name.clone()),
        display_name: Set(Some(user.display_name().to_string())),
        global_name: Set(user.global_name.clone()),
        nickname: Set(nickname),
        avatar: Set(user.avatar_url()),
        banner: Set(user.banner_url()),
        message_count: Set(0),
        voice_time: Set(0f64),
        total: Set(0f64),
        join_date: Set(Some(DateTimeWithTimeZone::from(user_join_date))),
        creation_date: Set(Some(DateTimeWithTimeZone::from(user_creation_date))),
        ..Default::default()
    };

    users::user_data::Entity::insert(user.clone())
        .exec_with_returning(db)
        .await
}
