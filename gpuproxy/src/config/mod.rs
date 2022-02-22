#[derive(Clone, Debug)]
pub enum Resource {
    Db,
    FS(String),
}

#[derive(Clone, Debug)]
pub struct ServiceConfig {
    pub url: String,
    pub db_dsn: String,
    pub disable_worker: bool,
    pub max_c2: usize,

    pub log_level: String,
    pub resource: Resource,
}

impl ServiceConfig {
    pub fn new(
        url: String,
        db_dsn: String,
        max_c2: usize,
        disable_worker: bool,
        resource_type: String,
        resource_path: String,
        log_level: String,
    ) -> Self {
        let resource = if resource_type == "db" {
            Resource::Db
        } else {
            Resource::FS(resource_path)
        };

        Self {
            url,
            db_dsn,
            max_c2,
            disable_worker,
            log_level,
            resource,
        }
    }
}

#[derive(Clone, Debug)]
pub struct ClientConfig {
    pub url: String,
    pub db_dsn: String,
    pub max_c2: usize,

    pub log_level: String,
    pub resource: Resource,
}

impl ClientConfig {
    pub fn new(url: String, db_dsn: String, max_c2: usize, resource_type: String, resource_path: String, log_level: String) -> Self {
        let resource = if resource_type == "db" {
            Resource::Db
        } else {
            Resource::FS(resource_path)
        };
        ClientConfig {
            url,
            db_dsn,
            max_c2,
            resource,
            log_level,
        }
    }
}
