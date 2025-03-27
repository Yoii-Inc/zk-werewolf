use serde::{Deserialize, Serialize};
use super::role::Role;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Player {
    pub id: String,
    pub name: String,
    pub role: Option<Role>,
    pub is_dead: bool,
}

impl Player {
    // この関数は将来のクライアントサイドでのプレイヤー作成時に使用予定
    // フロントエンドからの新規プレイヤー登録APIで使用することを想定
    #[allow(dead_code)]
    pub fn new(id: String, name: String, role: Option<Role>) -> Self {
        Self {
            id,
            name,
            role,
            is_dead: false,
        }
    }
}
