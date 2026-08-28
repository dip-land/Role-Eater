use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "users", table_name = "user_data")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub guild_id: String,
    pub username: String,
    pub display_name: Option<String>,
    pub global_name: Option<String>,
    pub nickname: Option<String>,
    pub avatar: Option<String>,
    pub banner: Option<String>,
    pub message_count: i64,
    #[sea_orm(column_type = "Double")]
    pub voice_time: f64,
    #[sea_orm(column_type = "Double")]
    pub total: f64,
    pub voice_channel_id: Option<String>,
    pub voice_channel_join_time: Option<String>,
    pub join_date: Option<DateTimeWithTimeZone>,
    pub creation_date: Option<DateTimeWithTimeZone>,
    #[sea_orm(default_value = "false")]
    pub user_left: bool,
    pub leave_date: Option<DateTimeWithTimeZone>,
    #[sea_orm(default_value = "14")]
    pub leave_deletion_duration: Decimal,
    #[sea_orm(default_value = "true")]
    pub message_data: bool,
    #[sea_orm(default_value = "true")]
    pub voice_data: bool,
    #[sea_orm(default_value = "true")]
    pub game_data: bool,
}

impl ActiveModelBehavior for ActiveModel {}
