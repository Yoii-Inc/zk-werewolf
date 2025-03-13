pub struct Rule {
    pub max_players: usize,
    pub min_players: usize,
    pub roles: Vec<String>,
    pub time_limit: Option<u32>,
}
