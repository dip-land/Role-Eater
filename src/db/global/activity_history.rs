use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "global", table_name = "activity_history")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub user_id: String,
    #[sea_orm(primary_key, auto_increment = false)]
    pub activity: i64,
    #[sea_orm(column_type = "Double")]
    pub time_played: f64,
}

impl ActiveModelBehavior for ActiveModel {}
