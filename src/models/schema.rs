table! {
    tasks (id) {
        id -> BigInt,
        miner -> Text,
        prove_id -> Text,
        sector_id -> BigInt,
        phase1_output -> Binary,
        proof -> Nullable<Binary>,
        status -> BigInt,
        create_at -> BigInt,
        complete_at -> Nullable<BigInt>,
    }
}
