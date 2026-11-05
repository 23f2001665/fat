pub const NO_EQUIPMENT_ID: i64 = 1;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Equipment {
    pub id: i64,
    pub name: String,
    pub category: Option<String>,
    pub is_active: bool,
}

impl Equipment {
    pub fn is_reserved(&self) -> bool {
        self.id == NO_EQUIPMENT_ID
    }

    pub fn active_label(&self) -> &'static str {
        if self.is_active {
            "Active"
        } else {
            "Inactive"
        }
    }
}
