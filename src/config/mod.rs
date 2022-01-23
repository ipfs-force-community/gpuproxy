#[derive(Clone, Debug)]
pub struct ServiceConfig  {
    pub url: String,
    pub db_dsn: String,
    pub disable_worker: bool,
    pub max_c2: usize
}


impl ServiceConfig {
    pub fn new(url: String, db_dsn: String, max_c2: usize, disable_worker: bool) -> Self {
        Self {
            url,
            db_dsn,
            max_c2,
            disable_worker,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub url: String,
    pub db_dsn: String,
    pub max_c2: usize
}

impl ClientConfig {
    pub fn new(url: String, db_dsn: String, max_c2: usize) -> Self {
        ClientConfig { url,db_dsn,max_c2 }
    }
}