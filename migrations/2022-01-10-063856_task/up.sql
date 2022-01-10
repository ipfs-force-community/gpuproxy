CREATE TABLE tasks (
                       id INTEGER NOT NULL PRIMARY KEY AUTOINCREMENT,
                       miner TEXT(256) NOT NULL,
                       prove_id TEXT(256) NOT NULL,
                       sector_id INTEGER NOT NULL,
                       phase1_output BLOB NOT NULL,
                       proof BLOB,
                       status INTEGER NOT NULL,
                       create_at INTEGER NOT NULL,
                       complete_at INTEGER
);
