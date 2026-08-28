use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[sea_orm::model]
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(schema_name = "global", table_name = "activity_data")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = true)]
    pub id: i32,
    #[sea_orm(primary_key, auto_increment = false)]
    pub name: String,
    pub alias: Option<Vec<String>>,
}

impl ActiveModelBehavior for ActiveModel {}
