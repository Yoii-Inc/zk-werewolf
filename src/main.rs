// #![allow(unused_imports)]
// #![allow(unused_variables)]

mod assign;

struct Player {
    name: String,
    sk: Option<SecretKey>,
    pk: Option<PublicKey>,
    role: Option<Role>,
}

type SecretKey = Vec<u32>;
type PublicKey = String;

impl Player {
    pub fn new(name: String) -> Self {
        //let (pk, sk) = keygen(rng);

        Player {
            name,
            sk: None, //後で変える
            pk: None, //後で変える
            role: Some(Role::Villager),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Role {
    Villager, //2
              // Seer,     //1
              // Hunter,   //1
              // Werewolf, //2
}

fn main() {
    let player1 = Player::new("Alice".to_string());

    assert_eq!(player1.name, "Alice");
    assert_eq!(player1.sk, None);
    assert_eq!(player1.pk, None);
    assert_eq!(player1.role, Some(Role::Villager));
}
