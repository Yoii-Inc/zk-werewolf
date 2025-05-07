use crate::models::node::NodeKey;
use std::collections::HashMap;
use std::sync::RwLock;

#[derive(Default)]
pub struct NodeKeyService {
    keys: RwLock<HashMap<u32, String>>,
}

impl NodeKeyService {
    pub fn new() -> Self {
        Self {
            keys: RwLock::new(HashMap::new()),
        }
    }

    pub fn store_key(&self, node_id: u32, public_key: String) -> Result<(), String> {
        if !(0..3).contains(&node_id) {
            return Err("Invalid node ID".to_string());
        }

        let mut keys = self.keys.write().unwrap();
        keys.insert(node_id, public_key);
        Ok(())
    }

    pub fn get_key(&self, node_id: u32) -> Option<String> {
        let keys = self.keys.read().unwrap();
        keys.get(&node_id).cloned()
    }

    pub fn get_all_keys(&self) -> Vec<NodeKey> {
        let keys = self.keys.read().unwrap();
        keys.iter()
            .map(|(&node_id, public_key)| NodeKey {
                node_id,
                public_key: public_key.clone(),
            })
            .collect()
    }
}
