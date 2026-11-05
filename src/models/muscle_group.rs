#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuscleGroup {
    pub id: i64,
    pub name: String,
    pub description: Option<String>,
    pub is_active: bool,
}

impl MuscleGroup {
    pub fn active_label(&self) -> &'static str {
        if self.is_active {
            "Active"
        } else {
            "Inactive"
        }
    }
}
