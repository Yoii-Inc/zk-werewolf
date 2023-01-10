#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(dead_code)]

use barnett_smart_card_protocol::discrete_log_cards;
use barnett_smart_card_protocol::BarnettSmartProtocol;

use ark_ff::{to_bytes, UniformRand};
use ark_std::{rand::Rng, One};
use itertools::Itertools;
use proof_essentials::utils::permutation::Permutation;
use proof_essentials::utils::rand::sample_vector;
use proof_essentials::zkp::proofs::{chaum_pedersen_dl_equality, schnorr_identification};
use rand::thread_rng;
use std::cmp::Ordering;
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

impl PartialOrd for WerewolfCard {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self {
            WerewolfCard::Player(val) => match other {
                WerewolfCard::Player(val_other) => Some(val.cmp(val_other)),
                WerewolfCard::Role(_) => Some(Ordering::Less),
            },
            WerewolfCard::Role(val) => match other {
                WerewolfCard::Player(_) => Some(Ordering::Greater),
                WerewolfCard::Role(val_other) => match (val, val_other) {
                    (WerewolfRole::Villager, WerewolfRole::Villager) => Some(Ordering::Equal),
                    (WerewolfRole::Villager, WerewolfRole::Seer) => Some(Ordering::Less),
                    (WerewolfRole::Villager, WerewolfRole::Hunter) => Some(Ordering::Less),
                    (WerewolfRole::Villager, WerewolfRole::Werewolf) => Some(Ordering::Less),
                    (WerewolfRole::Seer, WerewolfRole::Villager) => Some(Ordering::Greater),
                    (WerewolfRole::Seer, WerewolfRole::Seer) => Some(Ordering::Equal),
                    (WerewolfRole::Seer, WerewolfRole::Hunter) => Some(Ordering::Less),
                    (WerewolfRole::Seer, WerewolfRole::Werewolf) => Some(Ordering::Less),
                    (WerewolfRole::Hunter, WerewolfRole::Villager) => Some(Ordering::Greater),
                    (WerewolfRole::Hunter, WerewolfRole::Seer) => Some(Ordering::Greater),
                    (WerewolfRole::Hunter, WerewolfRole::Hunter) => Some(Ordering::Equal),
                    (WerewolfRole::Hunter, WerewolfRole::Werewolf) => Some(Ordering::Less),
                    (WerewolfRole::Werewolf, WerewolfRole::Villager) => Some(Ordering::Greater),
                    (WerewolfRole::Werewolf, WerewolfRole::Seer) => Some(Ordering::Greater),
                    (WerewolfRole::Werewolf, WerewolfRole::Hunter) => Some(Ordering::Greater),
                    (WerewolfRole::Werewolf, WerewolfRole::Werewolf) => Some(Ordering::Equal),
                },
            },
        }
    }
}

impl Ord for WerewolfCard {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
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

struct Num {
    players: usize, //参加プレイヤーの人数
    villagers: usize,
    seers: usize,
    hunters: usize,
    werewolfs: usize,
    groups: usize, //お互いに見える集団を1と数えた時の集団数(村、村、占、狩、狼)
    cards: usize,
    decks: usize, //必要なデッキ数。今は人狼の人数と同じになる。
}

impl Num {
    fn new(casting: [usize; 4]) -> Self {
        Self {
            players: casting.iter().sum(),
            villagers: casting[0],
            seers: casting[1],
            hunters: casting[2],
            werewolfs: casting[3],
            groups: casting[..3].iter().sum::<usize>() + 1,
            cards: casting.iter().sum::<usize>() + casting[..3].iter().sum::<usize>() + 1,
            decks: casting[3],
        }
    }
}

#[test]
fn test() -> anyhow::Result<()> {
    // セットアップ
    println!("セットアップ開始");

    let num = Num::new([3, 1, 1, 3]);

    let rng = &mut thread_rng();

    let parameters = CardProtocol::setup(rng, 2, 7)?; //ここは1を入れてもいいように変更が必要かもしれない．
    let card_mapping = encode_cards(rng, num.players, num.villagers, num.seers, num.hunters);

    // プレイヤー構築
    let player1 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Alice"].unwrap())?;
    let player2 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Bob"].unwrap())?;
    let player3 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Carol"].unwrap())?;
    let player4 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Dave"].unwrap())?;
    let player5 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Ellen"].unwrap())?;
    let player6 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Frank"].unwrap())?;
    let player7 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"George"].unwrap())?;
    let player8 = WerewolfPlayer::new(rng, &parameters, &to_bytes![b"Hall"].unwrap())?;

    let mut players = vec![
        player1, player2, player3, player4, player5, player6, player7, player8,
    ];

    let key_proof_info = players
        .iter()
        .map(|p| (p.pk, p.proof_key, p.name.clone()))
        .collect::<Vec<_>>();

    // Each player should run this computation. Alternatively, it can be ran by a smart contract
    let joint_pk = CardProtocol::compute_aggregate_key(&parameters, &key_proof_info)?;

    // プレイヤーカード、役カードの生成
    println!("プレイヤーカード、役カードの生成");
    //let card_mapping_keys_sorted = sort_card_mapping_keys(&card_mapping); //.collect();
    // card_mapping_vec.sort_by(|a, b| a.1.cmp(&b.1));
    // let deck_and_proofs: Vec<(MaskedCard, RemaskingProof)> = card_mapping
    //     .keys()
    //     .map(|card| CardProtocol::mask(rng, &parameters, &joint_pk, card, &Scalar::one()))
    //     .collect::<Result<Vec<_>, _>>()?;

    let vec: _ = card_mapping
        .iter()
        .sorted_by(|a, b| a.1.cmp(b.1))
        .collect_vec();
    let deck_and_proofs: Vec<(MaskedCard, RemaskingProof)> = card_mapping
        .iter()
        .sorted_by(|a, b| a.1.cmp(b.1))
        .map(|card| CardProtocol::mask(rng, &parameters, &joint_pk, card.0, &Scalar::one()))
        .collect::<Result<Vec<_>, _>>()?;

    let deck = deck_and_proofs
        .iter()
        .map(|x| x.0)
        .collect::<Vec<MaskedCard>>();

    // 各プレイヤーによるシャッフル
    println!("各プレイヤーによるシャッフル");
    //ここのPermutationのサイズをnにするか、もしくはサイズをm+nにして前半n個だけを置換するかどちらか。
    // TODO: partial shuffle
    //let permutation = Permutation::new(rng, num_of_cards);
    // let mut player_permutation = Permutation::new(rng, num_of_players);
    // player_permutation
    //     .mapping
    //     .extend([8, 9, 10, 11, 12, 13].iter());
    // let permutation = Permutation {
    //     mapping: player_permutation.mapping,
    //     size: num_of_cards,
    // };

    let mut partial_permutation = Permutation::new(rng, num.players);

    partial_permutation.mapping.extend(num.players..num.cards);

    let permutation = Permutation {
        mapping: partial_permutation.mapping,
        size: num.cards,
    };

    let mut inverse_map = vec![0; permutation.size];
    for i in 0..permutation.size {
        let j = permutation.mapping[i];
        inverse_map[j] = i;
    }

    let inverse_permutation = Permutation {
        mapping: inverse_map,
        size: num.cards,
    };

    let masking_factors: Vec<Scalar> = sample_vector(rng, num.cards);

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
    let whole_permutation = Permutation::from(&vec![8, 9, 10, 11, 12, 13, 5, 6, 0, 1, 2, 3, 4, 7]);
    //ここでは、 (0,8)(1,9)(2,10)(3,11)(4,12)(5,6,7,13)=[8,9,10,11,12,13,5,6,0,1,2,3,4,7]という置換を取っている。

    let mut decks = Vec::new();
    let mut deck_tmp = a_shuffled_deck_partial;

    for i in 0..num.decks {
        deck_tmp = Permutation::permute_array(&whole_permutation, &deck_tmp);
        decks.push(deck_tmp.clone());
    }

    // 各プレイヤーによるシャッフル
    decks = decks
        .iter()
        .map(|deck| Permutation::permute_array(&inverse_permutation, deck))
        .collect_vec();

    // シャッフルの検証
    // 配役の決定

    // 配役のデータの配布
    for i in 0..num.players {
        players[i].recieve_card(decks[0][i]);
    }

    //自分に配られた役職の確認

    let mut reveal_token: Vec<Vec<_>> = Vec::new();

    for p in players.iter() {
        let mut vec = Vec::new();
        for d in decks[0].iter() {
            vec.push(p.compute_reveal_token(rng, &parameters, d)?);
        }
        reveal_token.push(vec);
    }

    //(後で消す)カードの公開

    let mut rt = Vec::new();
    for i in 0..num.players {
        let mut vec = Vec::new();
        for j in 0..num.decks {
            let rt_part = players
                .iter()
                .map(|x| {
                    x.compute_reveal_token(rng, &parameters, &decks[j][i])
                        .unwrap()
                })
                .collect_vec();
            vec.push(rt_part);
        }
        rt.push(vec);
    }

    let mut players_card = Vec::new();

    for i in 0..num.players {
        let mut vec = Vec::new();
        for j in 0..num.decks {
            let x = open_werewolf_card(&parameters, &rt[i][j], &card_mapping, &decks[j][i])?;
            vec.push(x);
        }
        players_card.push(vec);
    }

    //役の表示
    for i in 0..num.players {
        for j in 0..num.decks {
            // println!(
            //     "{}: {:?}",
            //     std::str::from_utf8(&players[i].name).unwrap(),
            //     players_card[i][j]
            // );
            match players_card[i][j] {
                WerewolfCard::Player(_) => {}
                WerewolfCard::Role(role) => players[i].role = Some(role),
            }
        }
        println!(
            "{}: {:?}",
            std::str::from_utf8(&players[i].name).unwrap(),
            WerewolfCard::Role(players[i].role.unwrap())
        );
    }

    //TODO:以下のような表示にする
    //Alice: 役:占い, 同チーム:無し
    //Bob: 役:人狼, 同チーム:Dave

    // 次のステップへ移行
    Ok(())
}
