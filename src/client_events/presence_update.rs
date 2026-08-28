use crate::db::get_user;
use crate::{
    Bot,
    db::{global::*, users::*},
};
use chrono::prelude::*;
use sea_orm::ActiveValue::NotSet;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use serenity::model::gateway::Activity;
use serenity::prelude::*;
use serenity::{all::ActivityType, model::gateway::Presence};

const EXCLUDED_ACTIVITES: [&'static str; 4] = [
    "Curseforge",
    "CurseForge",
    "Screen Recording with Medal",
    "Valorant Tracker App",
];

pub async fn presence_update(bot: &Bot, ctx: Context, new_data: Presence) {
    let user = &new_data.user.to_user().unwrap();
    if user.bot || user.system {
        return;
    }

    let user = get_user(
        &bot.database,
        ctx.http.http(),
        &user,
        &new_data.guild_id.unwrap(),
    )
    .await;

    let user = match user {
        Ok(user) => user,
        Err(error) => return println!("Error creating user: {error:?}"),
    };

    // update activities after they end
    if new_data.activities.is_empty() {
        return println!("{:?} No Activities", new_data.user.name);
    }

    // add activities to user
    for activity in new_data.activities {
        if activity.kind != ActivityType::Playing
            || !user.game_data
            || activity.application_id.is_none()
        {
            continue;
        }

        if EXCLUDED_ACTIVITES.contains(&activity.name.as_str()) {
            continue;
        }

        let activity: activity_data::Model =
            find_or_create_activity(&activity, (&bot.database).clone()).await;

        println!(
            "{:?} User is playing {:?}_{:?}",
            new_data.user.name, activity.id, activity.name
        );
    }
}

pub async fn find_or_create_activity(
    activity: &Activity,
    database: DatabaseConnection,
) -> activity_data::Model {
    let activity_name = &activity.name;

    let activity_data: Option<activity_data::Model> = activity_data::Entity::find()
        .filter(activity_data::Column::Name.eq((&activity_name).to_string()))
        .one(&database)
        .await
        .unwrap();

    if activity_data.is_some() {
        return activity_data.unwrap();
    }

    let activity_data: Option<activity_data::Model> = activity_data::Entity::find()
        .filter(
            activity_data::COLUMN
                .alias
                .contains(vec![(&activity_name).to_string()]),
        )
        .one(&database)
        .await
        .unwrap();

    if activity_data.is_some() {
        return activity_data.unwrap();
    }

    let activity_data = activity_data::ActiveModel {
        id: NotSet,
        name: Set((&activity_name).to_string()),
        alias: NotSet,
    };

    println!("(DBUPDATE) ACTIVITY INSERT {:?}", &activity_name);

    activity_data::Entity::insert(activity_data)
        .exec_with_returning(&database)
        .await
        .unwrap()
}
