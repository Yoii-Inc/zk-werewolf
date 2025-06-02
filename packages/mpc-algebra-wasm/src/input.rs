use wasm_bindgen::prelude::wasm_bindgen;

#[wasm_bindgen]
pub struct AnonymousVotingInput {
    pub voter_id: u64,
    pub candidate_id: u64,
}

#[wasm_bindgen]
pub struct RoleAssignmentInput {
    pub role_id: u64,
    pub user_id: u64,
}

#[wasm_bindgen]
pub struct WinningJudgeInput {
    pub judge_id: u64,
    pub score: u64,
}

#[wasm_bindgen]
pub struct DivinationInput {}

#[wasm_bindgen]
pub struct KeyPublicizeInput {
    pub public_key: u8,
}
