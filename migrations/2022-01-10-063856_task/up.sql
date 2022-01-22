CREATE TABLE tasks (
                       id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                       miner TEXT(256) NOT NULL,
                       prove_id TEXT(256) NOT NULL,
                       sector_id INTEGER NOT NULL,
                       phase1_output Text NOT NULL,
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
                       id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                       worker_id TEXT(256) NOT NULL
                    
);
