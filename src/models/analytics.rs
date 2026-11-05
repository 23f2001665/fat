use chrono::NaiveDate;

use crate::utils::date::{DateRange, Period};

#[derive(Debug, Clone, PartialEq)]
pub struct SeriesPoint {
    pub date: NaiveDate,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExerciseAnalytics {
    pub exercise_id: Option<i64>,
    pub exercise_name: Option<String>,
    pub total_sets: i64,
    pub total_effective_reps: f64,
    pub total_volume: f64,
    pub max_weight: Option<f64>,
    pub average_weight: Option<f64>,
    pub average_rir: Option<f64>,
    pub weight_progression: Vec<SeriesPoint>,
    pub volume_progression: Vec<SeriesPoint>,
    pub rir_trend: Vec<SeriesPoint>,
}

impl ExerciseAnalytics {
    pub fn empty(exercise_id: Option<i64>, exercise_name: Option<String>) -> Self {
        Self {
            exercise_id,
            exercise_name,
            total_sets: 0,
            total_effective_reps: 0.0,
            total_volume: 0.0,
            max_weight: None,
            average_weight: None,
            average_rir: None,
            weight_progression: Vec::new(),
            volume_progression: Vec::new(),
            rir_trend: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MuscleGroupStats {
    pub muscle_group_id: i64,
    pub name: String,
    /// Sets where this muscle is a primary driver.
    pub primary_sets: i64,
    /// Sets where this muscle is a secondary contributor.
    pub secondary_sets: i64,
    /// Full set volume attributed to this muscle as a primary driver.
    pub primary_volume: f64,
    /// 50% of set volume attributed to this muscle as a secondary contributor.
    pub secondary_volume: f64,
    /// Distinct dates on which this muscle was trained.
    pub workout_days: i64,
}

impl MuscleGroupStats {
    pub fn total_volume_attributed(&self) -> f64 {
        self.primary_volume + self.secondary_volume
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct NamedValue {
    pub label: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalyticsReport {
    pub period: Period,
    pub range: DateRange,
    pub exercise: ExerciseAnalytics,
    pub muscle_groups: Vec<MuscleGroupStats>,
    pub weekly_volume: Vec<SeriesPoint>,
    pub muscle_distribution: Vec<NamedValue>,
    pub exercise_frequency: Vec<NamedValue>,
    pub has_data: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnalyticsChart {
    WeightProgression,
    VolumeProgression,
    WeeklyVolume,
    MuscleDistribution,
    ExerciseFrequency,
}

impl AnalyticsChart {
    pub fn from_key(value: &str) -> Self {
        match value {
            "volume" => Self::VolumeProgression,
            "weekly-volume" => Self::WeeklyVolume,
            "muscle" => Self::MuscleDistribution,
            "frequency" => Self::ExerciseFrequency,
            _ => Self::WeightProgression,
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Self::WeightProgression => "Weight progression",
            Self::VolumeProgression => "Volume progression",
            Self::WeeklyVolume => "Weekly training volume",
            Self::MuscleDistribution => "Attributed volume by muscle group",
            Self::ExerciseFrequency => "Exercise frequency (sets)",
        }
    }
}
