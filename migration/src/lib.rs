mod m20220101_000001_create_table;
mod m20220426_000001_create_table;
mod m20230111_000001_create_table;
mod m20230113_000001_create_table;

use sea_orm_migration::migrator::MigratorTrait;
use sea_orm_migration::MigrationTrait;

pub struct Migrator;

impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_create_table::Migration),
            Box::new(m20220426_000001_create_table::Migration),
            Box::new(m20230111_000001_create_table::Migration),
            Box::new(m20230113_000001_create_table::Migration),
        ]
    }
}
