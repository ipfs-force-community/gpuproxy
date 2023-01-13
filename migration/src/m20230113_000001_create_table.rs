use sea_orm_migration::prelude::*;
use sea_query::Table;

use entity::tasks as Tasks;

pub struct Migration;

impl MigrationName for Migration {
    fn name(&self) -> &str {
        "m20230113_000001_create_table"
    }
}

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .alter_table(
                Table::alter()
                    .table(Tasks::Entity)
                    .add_column(
                        ColumnDef::new(Tasks::Column::Comment)
                            .string()
                            .default("")
                            .not_null(),
                    )
                    .take(),
            )
            .await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        todo!()
    }
}
