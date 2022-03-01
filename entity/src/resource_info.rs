use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

// Resource Model, Every task needs to be calculated with a resource, and table is specially used to save the content of computing parameters
#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[sea_orm(table_name = "resource_infos")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false, column_type = "Text")]
    pub id: String,
    #[sea_orm(column_type = "Binary")]
    pub data: Vec<u8>,
    #[sea_orm(column_type = "Integer")]
    pub create_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
