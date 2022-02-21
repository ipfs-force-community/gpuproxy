use sea_schema::migration::prelude::*;

use entity::resource_info as ResourceInfo;
use entity::tasks as Tasks;
use entity::worker_info as WorkerInfos;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20220101_000001_create_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .create_table(
                Table::create()
                    .table(ResourceInfo::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ResourceInfo::Column::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(
                        ColumnDef::new(ResourceInfo::Column::Data)
                            .string()
                            .not_null(),
                    )
                    .col(
                        ColumnDef::new(ResourceInfo::Column::CreateAt)
                            .integer()
                            .not_null(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(WorkerInfos::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(WorkerInfos::Column::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .to_owned(),
            )
            .await?;

        manager
            .create_table(
                Table::create()
                    .table(Tasks::Entity)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(Tasks::Column::Id)
                            .string()
                            .not_null()
                            .primary_key(),
                    )
                    .col(ColumnDef::new(Tasks::Column::Miner).string().not_null())
                    .col(
                        ColumnDef::new(Tasks::Column::ResourceId)
                            .string()
                            .not_null(),
                    )
                    .col(ColumnDef::new(Tasks::Column::Proof).string().not_null())
                    .col(ColumnDef::new(Tasks::Column::WorkerId).string().not_null())
                    .col(ColumnDef::new(Tasks::Column::TaskType).integer().not_null())
                    .col(ColumnDef::new(Tasks::Column::ErrorMsg).string().not_null())
                    .col(ColumnDef::new(Tasks::Column::Status).integer().not_null())
                    .col(ColumnDef::new(Tasks::Column::CreateAt).integer().not_null())
                    .col(ColumnDef::new(Tasks::Column::StartAt).integer().not_null())
                    .col(
                        ColumnDef::new(Tasks::Column::CompleteAt)
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
