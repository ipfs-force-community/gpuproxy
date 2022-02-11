#[derive(Clone, Debug)]
pub struct ServiceConfig  {
    pub url: String,
    pub db_dsn: String,
    pub disable_worker: bool,
    pub max_c2: usize,

    pub log_level: String,
    pub resource_type: String,
    pub resource_path: String,
}


impl ServiceConfig {
    pub fn new(url: String, db_dsn: String, max_c2: usize, disable_worker: bool, resource_type: String, resource_path: String, log_level: String) -> Self {
        Self {
            url,
            db_dsn,
            max_c2,
            disable_worker,
            resource_type,
            resource_path,
            log_level,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub url: String,
    pub db_dsn: String,
    pub max_c2: usize,

    pub log_level: String,
    pub resource_type: String,
    pub resource_path: String,
}

impl ClientConfig {
    pub fn new(url: String, db_dsn: String, max_c2: usize, resource_type: String, resource_path: String, log_level: String) -> Self {
        ClientConfig {
            url,
            db_dsn,
            max_c2,
            resource_type,
            resource_path,
            log_level,
        }
    }
}