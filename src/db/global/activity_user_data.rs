use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "global", table_name = "activity_user_data")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: String,
    pub last_played_activity: Option<i32>,
    #[sea_orm(column_type = "Double", nullable)]
    pub last_played_time_played: Option<f64>,
    pub current_activity: Option<i32>,
    pub current_activity_start_time: Option<DateTime>,
}

impl ActiveModelBehavior for ActiveModel {}
