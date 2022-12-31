#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use barnett_smart_card_protocol::discrete_log_cards;
use barnett_smart_card_protocol::BarnettSmartProtocol;

use ark_ff::{to_bytes, UniformRand};
use ark_std::{rand::Rng, One};
use proof_essentials::utils::permutation::Permutation;
use proof_essentials::utils::rand::sample_vector;
use proof_essentials::zkp::proofs::{chaum_pedersen_dl_equality, schnorr_identification};
use rand::thread_rng;
use std::collections::HashMap;
use std::iter::Iterator;
use thiserror::Error;

//use crate::Player;

// Choose elliptic curve setting
type Curve = starknet_curve::Projective;
type Scalar = starknet_curve::Fr;

// Instantiate concrete type for our card protocol
type CardProtocol<'a> = discrete_log_cards::DLCards<'a, Curve>;
type CardParameters = discrete_log_cards::Parameters<Curve>;
type PublicKey = discrete_log_cards::PublicKey<Curve>;
type SecretKey = discrete_log_cards::PlayerSecretKey<Curve>;

type Card = discrete_log_cards::Card<Curve>;
type MaskedCard = discrete_log_cards::MaskedCard<Curve>;
type RevealToken = discrete_log_cards::RevealToken<Curve>;

type ProofKeyOwnership = schnorr_identification::proof::Proof<Curve>;
type RemaskingProof = chaum_pedersen_dl_equality::proof::Proof<Curve>;
type RevealProof = chaum_pedersen_dl_equality::proof::Proof<Curve>;

#[derive(Error, Debug, PartialEq)]
pub enum GameErrors {
    #[error("No such card in hand")]
    CardNotFound,

    #[error("Invalid card")]
    InvalidCard,
}

#[derive(PartialEq, Clone, Copy, Eq)]
pub enum WerewolfRole {
    Werewolf,
    Villager,
    Seer,
    Hunter,
}

// 配役用カード。プレイヤーカードと役職カードの二種類
#[derive(PartialEq, Clone, Copy, Eq)]

pub enum WerewolfCard {
    Role(WerewolfRole),
    Player(usize),
}

impl WerewolfCard {
    pub const fn unwrap_player(self) -> usize {
        match self {
            WerewolfCard::Player(val) => val,
            WerewolfCard::Role(_) => {
                panic!("called `WerewolfCard::unwrap_player()` on a `Role` value")
            }
        }
    }
    pub const fn unwrap_role(self) -> WerewolfRole {
        match self {
            WerewolfCard::Role(val) => val,
            WerewolfCard::Player(_) => {
                panic!("called `WerewolfCard::unwrap_role()` on a `Player` value")
            }
        }
    }
}

impl std::fmt::Debug for WerewolfCard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let result = match *self {
            WerewolfCard::Role(v) => match v {
                WerewolfRole::Villager => String::from("村人"),
                WerewolfRole::Seer => String::from("占い師"),
                WerewolfRole::Hunter => String::from("狩人"),
                WerewolfRole::Werewolf => String::from("人狼"),
            },
            WerewolfCard::Player(v) => v.to_string(),
        };
        write!(f, "{}", result)
    }
}

#[derive(Clone)]
struct WerewolfPlayer {
    name: Vec<u8>,
    sk: SecretKey,
    pk: PublicKey,
    proof_key: ProofKeyOwnership,
    cards: Vec<MaskedCard>,
    role: Option<WerewolfRole>,
}

impl WerewolfPlayer {
    pub fn new<R: Rng>(rng: &mut R, pp: &CardParameters, name: &Vec<u8>) -> anyhow::Result<Self> {
        let (pk, sk) = CardProtocol::player_keygen(rng, pp)?;
        let proof_key = CardProtocol::prove_key_ownership(rng, pp, &pk, &sk, name)?;
        Ok(Self {
            name: name.clone(),
            sk,
            pk,
            proof_key,
            role: None,
            cards: vec![],
        })
    }

    pub fn recieve_card(&mut self, card: MaskedCard) {
        // TODO
        self.role = Some(WerewolfRole::Villager); //card情報に応じて役職を決定する
    }

    pub fn peek_at_card(
        &mut self,
        parameters: &CardParameters,
        reveal_tokens: &mut Vec<(RevealToken, RevealProof, PublicKey)>,
        card_mappings: &HashMap<Card, WerewolfCard>,
        card: &MaskedCard,
    ) -> Result<(), anyhow::Error> {
        let i = self.cards.iter().position(|&x| x == *card);

        let i = i.ok_or(GameErrors::CardNotFound)?;

        //TODO add function to create that without the proof
        let rng = &mut thread_rng();
        let own_reveal_token = self.compute_reveal_token(rng, parameters, card)?;
        reveal_tokens.push(own_reveal_token);

        let unmasked_card = CardProtocol::unmask(parameters, reveal_tokens, card)?;
        let opened_card = card_mappings.get(&unmasked_card);
        let opened_card = opened_card.ok_or(GameErrors::InvalidCard)?;

        //self.opened_cards[i] = Some(*opened_card);
        Ok(())
    }

    pub fn compute_reveal_token<R: Rng>(
        &self,
        rng: &mut R,
        pp: &CardParameters,
        card: &MaskedCard,
    ) -> anyhow::Result<(RevealToken, RevealProof, PublicKey)> {
        let (reveal_token, reveal_proof) =
            CardProtocol::compute_reveal_token(rng, pp, &self.sk, &self.pk, card)?;

        Ok((reveal_token, reveal_proof, self.pk))
    }
}

// generate player & role cards.
fn encode_cards<R: Rng>(
    rng: &mut R,
    num_of_players: usize,
    num_of_villagers: usize,
    num_of_seers: usize,
    num_of_hunters: usize,
) -> HashMap<Card, WerewolfCard> {
    let mut map: HashMap<Card, WerewolfCard> = HashMap::new();
    // last +1 is werewolf group
    let num_of_role_cards = num_of_villagers + num_of_hunters + num_of_seers + 1;

    let plaintexts = (0..(num_of_players + num_of_role_cards))
        .map(|_| Card::rand(rng))
        .collect::<Vec<_>>();

    let mut i = 0;
    // Payer cards
    for p in 0..num_of_players {
        map.insert(plaintexts[i], WerewolfCard::Player(p));
        i += 1;
    }
    // Role cards (Don't know each others)
    for _ in 0..num_of_villagers {
        map.insert(plaintexts[i], WerewolfCard::Role(WerewolfRole::Villager));
        i += 1;
    }
    for _ in 0..num_of_hunters {
        map.insert(plaintexts[i], WerewolfCard::Role(WerewolfRole::Hunter));
        i += 1;
    }
    for _ in 0..num_of_seers {
        map.insert(plaintexts[i], WerewolfCard::Role(WerewolfRole::Seer));
        i += 1;
    }
    // Werewolf
    map.insert(plaintexts[i], WerewolfCard::Role(WerewolfRole::Werewolf));

    return map;
}

pub fn open_werewolf_card(
    parameters: &CardParameters,
    reveal_tokens: &Vec<(RevealToken, RevealProof, PublicKey)>,
    card_mappings: &HashMap<Card, WerewolfCard>,
    card: &MaskedCard,
) -> Result<WerewolfCard, anyhow::Error> {
    let unmasked_card = CardProtocol::unmask(parameters, reveal_tokens, card)?;
    let opened_card = card_mappings.get(&unmasked_card);
    let opened_card = opened_card.ok_or(GameErrors::InvalidCard)?;

    Ok(*opened_card)
}

#[test]
fn test() -> anyhow::Result<()> {
    // セットアップ
    println!("セットアップ開始");
    let num_of_players = 6; //参加プレイヤーの人数
    let m = [3, 1, 1, 1]; //配役の種類(村人，占い師，狩人，人狼)
    let num_of_cards = 12; // usize = m.iter().sum();
    let num_of_villagers = m[0];
    let num_of_seers = m[1];
    let num_of_hunters = m[2];
    //let num_of_werewolf = m[3];
    let num_of_groups = 6; //m[..3].iter().sum(); //お互いに見える集団を1と数えた時の集団数(村、村、占、狩、狼)

    let rng = &mut thread_rng();

    let parameters = CardProtocol::setup(rng, 2, 6)?; //ここは変更が必要かもしれない．
    let card_mapping = encode_cards(
        rng,
        num_of_players,
        num_of_villagers,
        num_of_seers,
        num_of_hunters,
    );

    // プレイヤー構築
    let player1 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Alice"].unwrap())?;
    let player2 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Bob"].unwrap())?;
    let player3 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Carol"].unwrap())?;
    let player4 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Dave"].unwrap())?;
    let player5 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Ellen"].unwrap())?;
    let player6 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Frank"].unwrap())?;

    let mut players = vec![player1, player2, player3, player4, player5, player6];

    let key_proof_info = players
        .iter()
        .map(|p| (p.pk, p.proof_key, p.name.clone()))
        .collect::<Vec<_>>();

    // Each player should run this computation. Alternatively, it can be ran by a smart contract
    let joint_pk = CardProtocol::compute_aggregate_key(&parameters, &key_proof_info)?;

    // プレイヤーカード、役カードの生成
    println!("プレイヤーカード、役カードの生成");
    let deck_and_proofs: Vec<(MaskedCard, RemaskingProof)> = card_mapping
        .keys()
        .map(|card| CardProtocol::mask(rng, &parameters, &joint_pk, card, &Scalar::one()))
        .collect::<Result<Vec<_>, _>>()?;

    let deck = deck_and_proofs
        .iter()
        .map(|x| x.0)
        .collect::<Vec<MaskedCard>>();

    // 各プレイヤーによるシャッフル
    println!("各プレイヤーによるシャッフル");
    //ここのPermutationのサイズをnにするか、もしくはサイズをm+nにして前半n個だけを置換するかどちらか。
    // TODO: partial shuffle
    let permutation = Permutation::new(rng, num_of_cards);
    let masking_factors: Vec<Scalar> = sample_vector(rng, num_of_cards);

    // let permutation = Permutation::new(rng, num_of_players);
    // let masking_factors: Vec<Scalar> = sample_vector(rng, num_of_players);

    println!("シャッフルアンドリマスクの開始");
    let (a_shuffled_deck_partial, a_shuffle_proof) = CardProtocol::shuffle_and_remask(
        rng,
        &parameters,
        &joint_pk,
        // 前半n個だけを取り出してシャッフル
        // &deck[0..num_of_players].to_vec(),
        &deck,
        &masking_factors,
        &permutation,
    )?;
    //.unwrap_or_else(|_| panic!("aaa"));

    // シャッフルの検証
    println!("シャッフルの検証");
    CardProtocol::verify_shuffle(
        &parameters,
        &joint_pk,
        // &deck[0..num_of_players].to_vec(),
        &deck,
        &a_shuffled_deck_partial,
        &a_shuffle_proof,
    )
    .unwrap_or_else(|_| panic!("vvv"));

    // グルーピングの為の全体における置換
    // 各プレイヤーによるシャッフル
    // シャッフルの検証
    // 配役の決定

    // 配役のデータの配布
    for i in 0..num_of_players {
        players[i].recieve_card(deck[i]);
    }

    //自分に配られた役職の確認

    // let mut reveal_token: vec![vec![(RevealToken, RevealProof, PublicKey); 11]; 6];
    let mut reveal_token: Vec<Vec<_>> = Vec::new();

    for p in players.iter() {
        let mut vec = Vec::new();
        for d in deck.iter() {
            vec.push(p.compute_reveal_token(rng, &parameters, d)?);
        }
        reveal_token.push(vec);
    }

    // let rts_player1 = vec![
    //     &reveal_token[1][0],
    //     &reveal_token[2][0],
    //     &reveal_token[3][0],
    //     &reveal_token[4][0],
    //     &reveal_token[5][0],
    // ];
    // let rts_player2 = vec![
    //     &reveal_token[0][1],
    //     &reveal_token[2][1],
    //     &reveal_token[3][1],
    //     &reveal_token[4][1],
    //     &reveal_token[5][1],
    // ];
    // let rts_player3 = vec![
    //     &reveal_token[0][2],
    //     &reveal_token[1][2],
    //     &reveal_token[3][2],
    //     &reveal_token[4][2],
    //     &reveal_token[5][2],
    // ];
    // let rts_player4 = vec![
    //     &reveal_token[0][3],
    //     &reveal_token[1][3],
    //     &reveal_token[2][3],
    //     &reveal_token[4][3],
    //     &reveal_token[5][3],
    // ];
    // let rts_player5 = vec![
    //     &reveal_token[0][4],
    //     &reveal_token[1][4],
    //     &reveal_token[2][4],
    //     &reveal_token[3][4],
    //     &reveal_token[5][4],
    // ];
    // let rts_player6 = vec![
    //     &reveal_token[0][5],
    //     &reveal_token[1][5],
    //     &reveal_token[2][5],
    //     &reveal_token[3][5],
    //     &reveal_token[4][5],
    // ];

    //(後で消す)カードの公開
    let rt_0 = vec![
        players[0].compute_reveal_token(rng, &parameters, &deck[0])?,
        players[1].compute_reveal_token(rng, &parameters, &deck[0])?,
        players[2].compute_reveal_token(rng, &parameters, &deck[0])?,
        players[3].compute_reveal_token(rng, &parameters, &deck[0])?,
        players[4].compute_reveal_token(rng, &parameters, &deck[0])?,
        players[5].compute_reveal_token(rng, &parameters, &deck[0])?,
    ];

    let player1_card = open_werewolf_card(&parameters, &rt_0, &card_mapping, &deck[0])?;

    //役の表示
    println!("Alice: {:?}", player1_card);

    // 次のステップへ移行
    Ok(())
}
