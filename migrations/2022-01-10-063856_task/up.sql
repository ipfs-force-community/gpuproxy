CREATE TABLE tasks (
                       id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                       miner TEXT(256) NOT NULL,
                       prove_id TEXT(256) NOT NULL,
                       sector_id INTEGER NOT NULL,
                       phase1_output Text NOT NULL,
                       proof Text,
                       worker_id Text,
                       task_type Integer,
                       error_msg Text,
                       status INTEGER NOT NULL,
                       create_at INTEGER NOT NULL,
                       start_at  -> INTEGER,
                       complete_at INTEGER
);

CREATE TABLE worker_infos (
                       id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                       worker_id TEXT(256) NOT NULL,
                    
);
