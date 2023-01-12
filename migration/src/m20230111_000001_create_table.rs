use sea_orm_migration::prelude::*;

use entity::workers_state as WorkersStates;
use log::warn;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20230111_000001_create_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(WorkersStates::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkersStates::Column::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(WorkersStates::Column::WorkerId)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkersStates::Column::Ips)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkersStates::Column::SupportTypes)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkersStates::Column::CreateAt)
                            .integer()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(WorkersStates::Column::UpdateAt)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        todo!()
    }
}

trait IgnoreExistDbResult {
    fn ignore_exist(self) -> Result<(), DbErr>;
}

impl IgnoreExistDbResult for Result<(), DbErr> {
    fn ignore_exist(self) -> Result<(), DbErr> {
        match self {
            Err(e) => {
                let e_str = e.to_string();
                if e_str.contains("Duplicate key name") {
                    warn!("ginore duplicate index {}", e_str);
                    Ok(())
                } else {
                    Err(e)
                }
            }
            _ => Ok(()),
        }
    }
}
