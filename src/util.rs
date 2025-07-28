pub fn make_stage_data_key(name: &str, rank: u8) -> String {
    format!("params:{rank}:{name}")
}
