use diesel::prelude::*;

embed_migrations!();


pub fn run_db_migrations(db: &SqliteConnection) -> std::result::Result<(), diesel_migrations::RunMigrationsError> {
    embedded_migrations::run_with_output(db, &mut std::io::stdout())
}
