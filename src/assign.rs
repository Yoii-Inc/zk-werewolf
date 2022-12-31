#![allow(unused_imports)]
#![allow(unused_variables)]

mod orig_assign;

#[cfg(test)]
mod tests {
    use rand::thread_rng;

    type Player = crate::Player;

    #[test]
    fn test() {
        // セットアップ
        let n = 6;
        let m = 4;
        // プレイヤー構築
        let player1 = Player::new("Alice".to_string());
        let player2 = Player::new("Bob".to_string());
        let player3 = Player::new("Carol".to_string());
        let player4 = Player::new("Dave".to_string());
        let player5 = Player::new("Ellen".to_string());
        let player6 = Player::new("Frank".to_string());
        // 配役の生成
        // 各プレイヤーによるシャッフル
        // シャッフルの検証
        // グルーピングの為の全体における置換
        // 各プレイヤーによるシャッフル
        // シャッフルの検証
        // 配役の決定
        // 配役のデータの配布
        // 次のステップへ移行
    }
}
