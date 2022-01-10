#[derive(Clone, Debug)]
pub struct ServiceConfig  {
    pub url: String,
    pub db_dsn: String,
}


impl ServiceConfig {
    pub fn new(url: String, db_dsn: String) -> Self {
        Self {
            url,
            db_dsn,
        }
    }
}
