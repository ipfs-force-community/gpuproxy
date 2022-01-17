#[derive(Clone, Debug)]
pub struct ServiceConfig  {
    pub url: String,
    pub db_dsn: String,
    pub disable_worker: bool
}


impl ServiceConfig {
    pub fn new(url: String, db_dsn: String, disable_worker: bool) -> Self {
        Self {
            url,
            db_dsn,
            disable_worker,
        }
    }
}
