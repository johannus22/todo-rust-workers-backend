use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct Todo {
    pub id: i64,
    pub title: String,
    pub completed: bool,
    pub created_at: String,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct AdminTodo {
    pub id: i64,
    pub title: String,
    pub completed: bool,
    pub created_at: String,
    pub owner_id: Option<String>,
    pub owner_email: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateTodo {
    pub title: String,
}

#[derive(Deserialize)]
pub struct UpdateTodo {
    pub completed: bool,
}
