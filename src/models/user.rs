#[derive(Debug, Clone, PartialEq)]
pub struct User {
    pub id: i64,
    pub name: String,
    pub height_cm: Option<f64>,
    pub date_of_birth: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}
