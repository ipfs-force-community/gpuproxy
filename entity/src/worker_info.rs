use sea_orm::entity::prelude::*;
use serde::{Serialize, Deserialize};


#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
#[sea_orm(table_name = "worker_infos")]
pub struct Model {
    #[sea_orm(primary_key)]
    #[sea_orm(column_type = "Text")]
    pub id: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}