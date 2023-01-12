use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

/// Worker model just used to save worker identity
#[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[sea_orm(table_name = "workers_states")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(unique)]
    pub worker_id: String,
    pub ips: String,
    pub support_types: String,
    pub update_at: i64,
    pub create_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
