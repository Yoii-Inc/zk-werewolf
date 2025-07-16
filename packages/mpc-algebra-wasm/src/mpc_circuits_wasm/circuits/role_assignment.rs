use crate::{RoleAssignmentPrivateInput, RoleAssignmentPublicInput};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RoleAssignmentCircuit {
    pub private_input: Vec<RoleAssignmentPrivateInput>,
    pub public_input: RoleAssignmentPublicInput,
}
