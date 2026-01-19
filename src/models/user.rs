use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct User {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize, Serialize)]
pub struct CreateUser {
    pub name: String,
}
