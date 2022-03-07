use entity::TaskType;

/// Resource type, there are 2 resource type for now,
/// this first is db resource that save task parameters in database,
/// and the second is file resource that save task parameters in file system
#[derive(Clone, Debug)]
pub enum Resource {
    Db,
    FS(String),
}

/// Save configuration information related to gpuproxyxw
#[derive(Clone, Debug)]
pub struct ServiceConfig {
    pub url: String,
    pub db_dsn: String,
    pub disable_worker: bool,
    pub max_tasks: usize,
    pub task_types: Option<Vec<TaskType>>,

    pub log_level: String,
    pub resource: Resource,
}

impl ServiceConfig {
    pub fn new(
        url: String,
        db_dsn: String,
        max_tasks: usize,
        disable_worker: bool,
        resource_type: String,
        resource_path: String,
        log_level: String,
        task_types: Option<Vec<TaskType>>,
    ) -> Self {
        let resource = if resource_type == "db" {
            Resource::Db
        } else {
            Resource::FS(resource_path)
        };

        Self {
            url,
            db_dsn,
            max_tasks,
            disable_worker,
            log_level,
            resource,
            task_types,
        }
    }
}

/// Save configuration information related to gpuproxy worker
#[derive(Clone, Debug)]
pub struct WorkerConfig {
    pub url: String,
    pub db_dsn: String,
    pub max_tasks: usize,
    pub task_types: Option<Vec<TaskType>>,

    pub log_level: String,
    pub resource: Resource,
}

impl WorkerConfig {
    pub fn new(
        url: String,
        db_dsn: String,
        max_tasks: usize,
        resource_type: String,
        resource_path: String,
        log_level: String,
        task_types: Option<Vec<TaskType>>,
    ) -> Self {
        let resource = if resource_type == "db" {
            Resource::Db
        } else {
            Resource::FS(resource_path)
        };
        WorkerConfig {
            url,
            db_dsn,
            max_tasks,
            resource,
            log_level,
            task_types,
        }
    }
}
