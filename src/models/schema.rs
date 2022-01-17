table! {
    tasks (id) {
        id -> BigInt,
        miner -> Text,
        prove_id -> Text,
        sector_id -> BigInt,
        phase1_output -> Text,
        proof -> Text,
        task_type -> Integer,
        error_msg -> Text,
        status -> Integer,
        create_at -> BigInt,
        start_at  -> BigInt,
        complete_at -> BigInt,
    }
}