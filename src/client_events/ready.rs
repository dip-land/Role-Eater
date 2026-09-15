use crate::Bot;
use serenity::model::gateway::Ready;
use serenity::prelude::*;

#[cfg(not(debug_assertions))]
use crate::db::{global::*, users::*};
#[cfg(not(debug_assertions))]
use chrono::{DateTime, NaiveDate};
#[cfg(not(debug_assertions))]
use mongodb::{
    Client, Collection,
    bson::{Document, doc},
};
#[cfg(not(debug_assertions))]
use sea_orm::{ActiveValue::NotSet, DbErr, InsertResult, sea_query::OnConflict};
#[cfg(not(debug_assertions))]
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
#[cfg(not(debug_assertions))]
use serenity::all::GuildId;
#[cfg(not(debug_assertions))]
use serenity::{all::UserId, futures::TryStreamExt};
#[cfg(not(debug_assertions))]
use std::collections::HashMap;
#[cfg(not(debug_assertions))]
use tokio_cron_scheduler::{Job, JobScheduler};

pub struct ParsedActivity {
    pub id: i32,
    pub time_played: f64,
}

pub async fn ready(bot: &Bot, ctx: Context, data_about_bot: Ready) {
    if cfg!(debug_assertions) {
        println!("v.{}\n{:?}", bot.version, data_about_bot);
    } else {
        println!("v.{} ONLINE", bot.version);
        #[cfg(not(debug_assertions))]
        scheduler(bot.clone(), ctx.clone()).await;
    }
}

#[cfg(not(debug_assertions))]
pub async fn scheduler(bot: &Bot, ctx: Context) {
    let mut sched = JobScheduler::new().await.unwrap();

    let bot = bot.clone();
    let ctx = ctx.clone();

    sched
        .add(
            Job::new_async("0 */15 * * * *", move |_uuid, _lock| {
                let bot = bot.clone();
                let ctx = ctx.clone();

                Box::pin(async move {
                    if let Err(error) = run(&bot, ctx).await {
                        eprintln!("Scheduled activity update failed: {error}");
                    }
                })
            })
            .unwrap(),
        )
        .await
        .unwrap();

    sched.shutdown_on_ctrl_c();

    sched.set_shutdown_handler(Box::new(|| {
        Box::pin(async move {
            println!("Jobs Shut Down!");
        })
    }));

    sched.start().await.unwrap();
}

#[cfg(not(debug_assertions))]
async fn run(bot: &Bot, ctx: Context) -> mongodb::error::Result<()> {
    let uri = "mongodb://192.168.1.51:27017";
    let client = Client::with_uri_str(uri).await.unwrap();
    let mdb = client.database("DipLand");

    let collection: Collection<Document> = mdb.collection(&"1110754252315435070");
    let cursor = collection.find(doc! {}).await.unwrap();
    let all: Vec<_> = cursor.try_collect().await?;
    for doc in &all {
        let id = doc.get_str("id").unwrap();
        let guild_id = "1110754252315435070";
        if id == guild_id {
            println!("Skipped: {:?}", id);
            continue;
        }

        let guild = GuildId::new(guild_id.to_string().parse().unwrap());
        let user = UserId::new(id.to_string().parse().unwrap());
        let guild_member = match guild.member(&ctx.http, &user).await {
            Ok(member) => Some(member),
            Err(_) => {
                println!("{} / {}", i, all.len() - 1);
                i = i + 1;
                continue;
            }
        };
        let fallback_username = doc.get_str("username").unwrap();

        let total = doc.get("total").unwrap();
        let total = match total.as_f64() {
            Some(total) => total,
            None => f64::from(total.as_i32().unwrap()),
        };
        let message = doc.get_document("message").unwrap();
        let message_count = message.get_i32("count").unwrap();

        let voice = doc.get("voice").unwrap().as_document().unwrap();
        let current_voice_channel_id = voice.get("channelID").unwrap();
        let current_voice_channel_id = match current_voice_channel_id.as_str() {
            Some(current_voice_channel_id) => Some(current_voice_channel_id.to_string()),
            None => None,
        };
        let joined_voice_channel_time = voice.get("lastJoinDate").unwrap();
        let joined_voice_channel_time = match joined_voice_channel_time.as_str() {
            Some(joined_voice_channel_time) => Some(joined_voice_channel_time.to_string()),
            None => None,
        };
        let voice_time = voice.get("time").unwrap();
        let voice_time = match voice_time.as_f64() {
            Some(voice_time) => voice_time,
            None => f64::from(voice_time.as_i32().unwrap()),
        };

        let username = match &guild_member {
            Some(member) => member.user.name.clone(),
            None => fallback_username.to_string(),
        };
        let display_name = match &guild_member {
            Some(member) => Set(Some(member.display_name().to_string())),
            None => NotSet,
        };
        let global_name = match &guild_member {
            Some(member) => Set(member.user.global_name.clone()),
            None => NotSet,
        };
        let nickname = match &guild_member {
            Some(member) => Set(member.nick.clone()),
            None => NotSet,
        };
        let pre_avatar = match doc.get("avatar").unwrap().as_str() {
            Some(avatar) => Some(avatar.to_string()),
            None => None,
        };
        let avatar = match &guild_member {
            Some(member) => match member.avatar_url() {
                Some(avatar) => Set(Some(avatar)),
                None => match member.user.avatar_url() {
                    Some(avatar) => Set(Some(avatar)),
                    None => Set(pre_avatar),
                },
            },
            None => NotSet,
        };
        let banner = match &guild_member {
            Some(member) => match member.banner_url() {
                Some(banner) => Set(Some(banner)),
                None => Set(member.user.banner_url()),
            },
            None => NotSet,
        };
        let join_date = match &guild_member {
            Some(member) => Set(Some(member.joined_at.unwrap().fixed_offset())),
            None => NotSet,
        };
        let creation_date = match &guild_member {
            Some(member) => Set(Some(member.user.created_at().fixed_offset())),
            None => NotSet,
        };

        let user_left = match &guild_member {
            Some(_) => Set(false),
            None => Set(true),
        };

        let user_data = user_data::ActiveModel {
            user_id: Set((&id).to_string()),
            guild_id: Set(guild_id.to_string()),
            username: Set(username),
            display_name: display_name,
            global_name: global_name,
            nickname: nickname,
            avatar: avatar,
            banner: banner,
            message_count: Set(message_count as i64),
            voice_time: Set(voice_time),
            total: Set(total),
            voice_channel_id: Set(current_voice_channel_id),
            voice_channel_join_time: Set(joined_voice_channel_time),
            join_date: join_date,
            creation_date: creation_date,
            user_left: user_left,
            leave_date: NotSet,
            leave_deletion_duration: Set(365),
            message_data: NotSet,
            voice_data: NotSet,
            game_data: NotSet,
        };

        let _ = user_data::Entity::insert(user_data)
            .on_conflict(
                OnConflict::columns([user_data::Column::UserId, user_data::Column::GuildId])
                    .update_columns([
                        user_data::Column::Username,
                        user_data::Column::DisplayName,
                        user_data::Column::GlobalName,
                        user_data::Column::Nickname,
                        user_data::Column::Avatar,
                        user_data::Column::Banner,
                        user_data::Column::MessageCount,
                        user_data::Column::VoiceTime,
                        user_data::Column::Total,
                        user_data::Column::VoiceChannelId,
                        user_data::Column::VoiceChannelJoinTime,
                    ])
                    .to_owned(),
            )
            .exec(&bot.database)
            .await;

        let message_history = message.get_array("history").unwrap();

        for history in message_history {
            let history = history.as_document().unwrap();
            let date = history.get_str("date").unwrap();
            let date = NaiveDate::parse_from_str(date, "%a %b %d %Y").unwrap();
            let count = history.get_i32("count").unwrap();

            let voice_message_history = voice_message_history::ActiveModel {
                user_id: Set((&id).to_string()),
                guild_id: Set((guild_id).to_string()),
                date: Set(date),
                message_count: Set(count as i64),
                voice_time: Set(0.0),
            };

            let _ = voice_message_history::Entity::insert(voice_message_history)
                .on_conflict(
                    OnConflict::columns([
                        voice_message_history::Column::UserId,
                        voice_message_history::Column::GuildId,
                        voice_message_history::Column::Date,
                    ])
                    .update_column(voice_message_history::Column::MessageCount)
                    .to_owned(),
                )
                .exec(&bot.database)
                .await;
        }

        let voice_history = voice.get_array("history").unwrap();

        for history in voice_history {
            let history = history.as_document().unwrap();
            let date = history.get_str("date").unwrap();
            let date = NaiveDate::parse_from_str(date, "%a %b %d %Y").unwrap();
            let time = match history.get("time").unwrap().as_f64() {
                Some(time) => time,
                None => f64::from(history.get_i32("time").unwrap()),
            };

            let voice_message_history = voice_message_history::ActiveModel {
                user_id: Set((&id).to_string()),
                guild_id: Set((guild_id).to_string()),
                date: Set(date),
                message_count: Set(0),
                voice_time: Set(time),
            };

            let _ = voice_message_history::Entity::insert(voice_message_history)
                .on_conflict(
                    OnConflict::columns([
                        voice_message_history::Column::UserId,
                        voice_message_history::Column::GuildId,
                        voice_message_history::Column::Date,
                    ])
                    .update_column(voice_message_history::Column::VoiceTime)
                    .to_owned(),
                )
                .exec(&bot.database)
                .await;
        }

        for history in message_history {
            let history = history.as_document().unwrap();
            let date = history.get_str("date").unwrap();
            let date = NaiveDate::parse_from_str(date, "%a %b %d %Y").unwrap();
            let count = history.get_i32("count").unwrap();

            let voice_message_history = voice_message_history::ActiveModel {
                user_id: Set((&id).to_string()),
                guild_id: Set((guild_id).to_string()),
                date: Set(date),
                message_count: Set(count as i64),
                voice_time: NotSet,
            };

            let _ = voice_message_history::Entity::insert(voice_message_history)
                .on_conflict(
                    OnConflict::columns([
                        voice_message_history::Column::UserId,
                        voice_message_history::Column::GuildId,
                        voice_message_history::Column::Date,
                    ])
                    .update_column(voice_message_history::Column::MessageCount)
                    .to_owned(),
                )
                .exec(&bot.database)
                .await;
        }

        let activities = doc.get("activities").unwrap().as_document().unwrap();
        let game_activities = activities.get("game").unwrap().as_document().unwrap();
        let game_currently_playing = game_activities.get("title").unwrap();
        let game_currently_playing = match game_currently_playing.as_str() {
            Some(game_currently_playing) => Some(game_currently_playing),
            None => None,
        };
        let game_currently_playing_start_time = game_activities.get("startDate").unwrap();
        let game_currently_playing_start_time = match game_currently_playing_start_time.as_i64() {
            Some(game_currently_playing_start_time) => Some(game_currently_playing_start_time),
            None => match game_currently_playing_start_time.as_f64() {
                Some(game_currently_playing_start_time) => {
                    Some(game_currently_playing_start_time.round() as i64)
                }
                None => None,
            },
        };
        let game_last_played = game_activities.get("lastPlayed").unwrap();
        let game_last_played = match game_last_played.as_str() {
            Some(game_last_played) => Some(game_last_played),
            None => None,
        };
        let game_last_played_time = game_activities.get("lastPlayedTime").unwrap();
        let game_last_played_time = match game_last_played_time.as_f64() {
            Some(game_last_played_time) => Some(game_last_played_time),
            None => match game_last_played_time.as_i32() {
                Some(game_last_played_time) => Some(f64::from(game_last_played_time)),
                None => None,
            },
        };
        let game_history = game_activities.get_array("history").unwrap();
        let game_time_history = game_activities.get_array("timeHistory").unwrap();

        let mut parsed_activites: HashMap<i32, f64> = HashMap::new();
        for activity in game_history {
            let activity = activity.as_document().unwrap();
            let title = activity.get_str("title").unwrap();
            if title == "Rainbow Six Siege" {
                continue;
            }
            let time = match activity.get("time").unwrap().as_f64() {
                Some(time) => time,
                None => f64::from(activity.get_i32("time").unwrap()),
            };
            let activity = find_or_create_activity(title, (&bot.database).clone()).await;
            match parsed_activites.contains_key(&activity.id) {
                true => {
                    *parsed_activites.get_mut(&activity.id).unwrap() += time;
                }
                false => {
                    parsed_activites.insert(activity.id, time);
                }
            }
        }

        for (k, v) in parsed_activites {
            let _ = insert_or_update_activity_history(id, k, v, (&bot.database).clone()).await;
        }

        let last_played_activity = match game_last_played {
            Some(title) => Some(i64::from(
                find_or_create_activity(title, (&bot.database).clone())
                    .await
                    .id,
            )),
            None => None,
        };

        let current_activity = match game_currently_playing {
            Some(title) => Some(i64::from(
                find_or_create_activity(title, (&bot.database).clone())
                    .await
                    .id,
            )),
            None => None,
        };

        let current_activity_start_time = match game_currently_playing_start_time {
            Some(ts) => Some(DateTime::from_timestamp_millis(ts).unwrap().naive_utc()),
            None => None,
        };

        let activity_user_data_active = activity_user_data::ActiveModel {
            user_id: Set((&id).to_string()),
            last_played_activity: Set(last_played_activity),
            last_played_time_played: Set(game_last_played_time),
            current_activity: Set(current_activity),
            current_activity_start_time: Set(current_activity_start_time),
        };

        let _ = activity_user_data::Entity::insert(activity_user_data_active)
            .on_conflict(
                OnConflict::column(activity_user_data::Column::UserId)
                    .update_columns([
                        activity_user_data::Column::LastPlayedActivity,
                        activity_user_data::Column::LastPlayedTimePlayed,
                        activity_user_data::Column::CurrentActivity,
                        activity_user_data::Column::CurrentActivityStartTime,
                    ])
                    .to_owned(),
            )
            .exec(&bot.database)
            .await;

        for activity in game_time_history {
            let activity = activity.as_document().unwrap();
            let date = activity.get_str("date").unwrap();
            let date = NaiveDate::parse_from_str(date, "%a %b %d %Y").unwrap();
            let time = match activity.get("time").unwrap().as_f64() {
                Some(time) => time,
                None => f64::from(activity.get_i32("time").unwrap()),
            };

            let activity_time_history = activity_time_history::ActiveModel {
                user_id: Set((&id).to_string()),
                date: Set(date),
                time_played: Set(time),
            };

            let _ = activity_time_history::Entity::insert(activity_time_history)
                .on_conflict(
                    OnConflict::columns([
                        activity_time_history::Column::UserId,
                        activity_time_history::Column::Date,
                    ])
                    .update_column(activity_time_history::Column::TimePlayed)
                    .to_owned(),
                )
                .exec(&bot.database)
                .await;
        }
    }
    println!("SYNC COMPLETE");
    Ok(())
}

#[cfg(not(debug_assertions))]
pub async fn find_or_create_activity(
    activity: &str,
    database: DatabaseConnection,
) -> activity_data::Model {
    let activity_name = &activity;

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

#[cfg(not(debug_assertions))]
pub async fn insert_or_update_activity_history(
    uid: &str,
    activity: i32,
    time_played: f64,
    database: DatabaseConnection,
) -> Result<InsertResult<activity_history::ActiveModel>, DbErr> {
    let activity_history = activity_history::ActiveModel {
        user_id: Set((&uid).to_string()),
        activity: Set(i64::from(activity)),
        time_played: Set(time_played),
    };

    activity_history::Entity::insert(activity_history)
        .on_conflict(
            OnConflict::columns([
                activity_history::Column::UserId,
                activity_history::Column::Activity,
            ])
            .update_column(activity_history::Column::TimePlayed)
            .to_owned(),
        )
        .exec(&database)
        .await
}
