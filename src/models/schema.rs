table! {
    tasks (id) {
        id -> Text,
        miner -> Text,
        resource_id -> Text,
        proof -> Text,
        worker_id -> Text,
        task_type -> Integer,
        error_msg -> Text,
        status -> Integer,
        create_at -> BigInt,
        start_at  -> BigInt,
        complete_at -> BigInt,
    }
}


table! {
    worker_infos (id) {
        id -> Text,
    }
}

table! {
    resource_infos (id) {
        id -> Text,
        data -> Binary,
        create_at -> BigInt,
    }
}

allow_tables_to_appear_in_same_query!(
    tasks,
    worker_infos,
    resource_infos,
);
