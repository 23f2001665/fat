use chrono::NaiveDate;

use crate::utils::calculations::{effective_reps, set_volume};

#[derive(Debug, Clone, PartialEq)]
pub struct WorkoutSet {
    pub date: NaiveDate,
    pub sequence_number: i64,
    pub exercise_id: i64,
    pub exercise_name: String,
    pub equipment_id: i64,
    pub equipment_name: String,
    pub weight: f64,
    pub complete_reps: i64,
    pub partial_reps: i64,
    pub rest_seconds: Option<i64>,
    pub rir: Option<i64>,
    pub extra_info: Option<String>,
}

impl WorkoutSet {
    pub fn effective_reps(&self) -> f64 {
        effective_reps(self.complete_reps, self.partial_reps)
    }

    pub fn volume(&self) -> f64 {
        set_volume(self.weight, self.complete_reps, self.partial_reps)
    }
}

#[derive(Debug, Clone)]
pub struct NewWorkoutSet {
    pub date: NaiveDate,
    pub exercise_id: i64,
    pub equipment_id: i64,
    pub weight: f64,
    pub complete_reps: i64,
    pub partial_reps: i64,
    pub rest_seconds: Option<i64>,
    pub rir: Option<i64>,
    pub extra_info: Option<String>,
}

/// Changes to an existing set. Date and sequence number are immutable so that
/// chronological identity is stable; deleting a set never renumbers the rest.
#[derive(Debug, Clone)]
pub struct UpdateWorkoutSet {
    pub exercise_id: i64,
    pub equipment_id: i64,
    pub weight: f64,
    pub complete_reps: i64,
    pub partial_reps: i64,
    pub rest_seconds: Option<i64>,
    pub rir: Option<i64>,
    pub extra_info: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorkoutDaySummary {
    pub date: NaiveDate,
    pub set_count: i64,
    pub total_volume: f64,
    pub exercise_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct WorkoutDay {
    pub date: NaiveDate,
    pub sets: Vec<WorkoutSet>,
}

impl WorkoutDay {
    pub fn total_volume(&self) -> f64 {
        self.sets.iter().map(WorkoutSet::volume).sum()
    }

    pub fn exercise_names(&self) -> Vec<String> {
        let mut names: Vec<String> = Vec::new();
        for set in &self.sets {
            if !names.contains(&set.exercise_name) {
                names.push(set.exercise_name.clone());
            }
        }
        names
    }
}
