CREATE TABLE tasks (
                       id TEXT(256) NOT NULL PRIMARY KEY,
                       miner TEXT(256) NOT NULL,
                       resource_id Text NOT NULL,
                       proof Text NOT NULL DEFAULT "",
                       worker_id Text NOT NULL  DEFAULT "",
                       task_type Integer NOT NULL  DEFAULT 0,
                       error_msg Text NOT NULL  DEFAULT "",
                       status INTEGER NOT NULL DEFAULT 0,
                       create_at INTEGER NOT NULL DEFAULT 0,
                       start_at   INTEGER NOT NULL DEFAULT 0,
                       complete_at INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE worker_infos (
                       id TEXT(256) NOT NULL PRIMARY KEY       
);

CREATE TABLE resource_infos (
    id TEXT(256) NOT NULL PRIMARY KEY,
    data Blob NOT NULL  DEFAULT 0,
    create_at INTEGER NOT NULL DEFAULT 0
);
