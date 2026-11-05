use serde::{Deserialize, Serialize};

use crate::error::{AppError, AppResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExerciseCategory {
    Compound,
    Isolation,
    Bodyweight,
    Other,
}

impl ExerciseCategory {
    pub const ALL: [Self; 4] = [
        Self::Compound,
        Self::Isolation,
        Self::Bodyweight,
        Self::Other,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Compound => "Compound",
            Self::Isolation => "Isolation",
            Self::Bodyweight => "Bodyweight",
            Self::Other => "Other",
        }
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "Compound" => Ok(Self::Compound),
            "Isolation" => Ok(Self::Isolation),
            "Bodyweight" => Ok(Self::Bodyweight),
            "Other" => Ok(Self::Other),
            other => Err(AppError::validation(format!(
                "Unknown exercise category '{other}'"
            ))),
        }
    }

    pub fn from_ui(value: &str) -> Self {
        match value {
            "Compound" => Self::Compound,
            "Isolation" => Self::Isolation,
            "Bodyweight" => Self::Bodyweight,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum MuscleRole {
    Primary,
    Secondary,
}

impl MuscleRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Primary => "Primary",
            Self::Secondary => "Secondary",
        }
    }

    pub fn from_db(value: &str) -> AppResult<Self> {
        match value {
            "Primary" => Ok(Self::Primary),
            "Secondary" => Ok(Self::Secondary),
            other => Err(AppError::validation(format!(
                "Unknown muscle role '{other}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExerciseMuscle {
    pub muscle_group_id: i64,
    pub muscle_group_name: String,
    pub role: MuscleRole,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Exercise {
    pub id: i64,
    pub name: String,
    pub normalized_name: String,
    pub description: Option<String>,
    pub category: ExerciseCategory,
    pub is_active: bool,
    pub muscles: Vec<ExerciseMuscle>,
}

impl Exercise {
    pub fn muscle_summary(&self) -> String {
        if self.muscles.is_empty() {
            return "No muscle groups assigned".to_string();
        }
        self.muscles
            .iter()
            .map(|muscle| {
                let marker = match muscle.role {
                    MuscleRole::Primary => "P",
                    MuscleRole::Secondary => "S",
                };
                format!("{} ({marker})", muscle.muscle_group_name)
            })
            .collect::<Vec<_>>()
            .join(", ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MuscleAssignment {
    pub muscle_group_id: i64,
    pub role: MuscleRole,
}

#[derive(Debug, Clone, Default)]
pub struct ExerciseInput {
    pub name: String,
    pub description: String,
    pub category: ExerciseCategoryInput,
    pub is_active: bool,
    pub muscles: Vec<MuscleAssignment>,
}

/// UI-friendly category. The repository/service validates and converts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExerciseCategoryInput {
    Compound,
    Isolation,
    Bodyweight,
    #[default]
    Other,
}

impl ExerciseCategoryInput {
    pub fn from_ui(value: &str) -> Self {
        match value {
            "Compound" => Self::Compound,
            "Isolation" => Self::Isolation,
            "Bodyweight" => Self::Bodyweight,
            _ => Self::Other,
        }
    }

    pub fn into_category(self) -> ExerciseCategory {
        match self {
            Self::Compound => ExerciseCategory::Compound,
            Self::Isolation => ExerciseCategory::Isolation,
            Self::Bodyweight => ExerciseCategory::Bodyweight,
            Self::Other => ExerciseCategory::Other,
        }
    }
}
