use sea_schema::migration::prelude::*;

use entity::tasks as Tasks;
use log::warn;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220426_000001_create_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_index(
                sea_query::Index::create()
                    .table(Tasks::Entity)
                    .name("tasks_create_at")
                    .col(Tasks::Column::CreateAt)
                    .index_type(IndexType::BTree)
                    .take(),
            )
            .await
            .ignore_exist()?;

        manager
            .create_index(
                sea_query::Index::create()
                    .table(Tasks::Entity)
                    .name("tasks_start_at")
                    .col(Tasks::Column::StartAt)
                    .index_type(IndexType::BTree)
                    .take(),
            )
            .await
            .ignore_exist()?;

        manager
            .create_index(
                sea_query::Index::create()
                    .table(Tasks::Entity)
                    .name("tasks_complete_at")
                    .col(Tasks::Column::CompleteAt)
                    .index_type(IndexType::BTree)
                    .take(),
            )
            .await
            .ignore_exist()?;

        manager
            .create_index(
                sea_query::Index::create()
                    .table(Tasks::Entity)
                    .name("tasks_state")
                    .col(Tasks::Column::State)
                    .index_type(IndexType::BTree)
                    .take(),
            )
            .await
            .ignore_exist()?;

        manager
            .create_index(
                sea_query::Index::create()
                    .table(Tasks::Entity)
                    .name("tasks_tasktype")
                    .col(Tasks::Column::TaskType)
                    .index_type(IndexType::BTree)
                    .take(),
            )
            .await
            .ignore_exist()?;

        manager
            .create_index(
                sea_query::Index::create()
                    .table(Tasks::Entity)
                    .name("tasks_worker_id")
                    .col(Tasks::Column::WorkerId)
                    .index_type(IndexType::BTree)
                    .take(),
            )
            .await
            .ignore_exist()?;

        manager
            .create_index(
                sea_query::Index::create()
                    .table(Tasks::Entity)
                    .name("tasks_workerid_state")
                    .col(Tasks::Column::WorkerId)
                    .col(Tasks::Column::State)
                    .index_type(IndexType::BTree)
                    .take(),
            )
            .await
            .ignore_exist()?;

        manager
            .create_index(
                sea_query::Index::create()
                    .table(Tasks::Entity)
                    .name("tasks_workerid_tasktype")
                    .col(Tasks::Column::WorkerId)
                    .col(Tasks::Column::TaskType)
                    .index_type(IndexType::BTree)
                    .take(),
            )
            .await
            .ignore_exist()
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
