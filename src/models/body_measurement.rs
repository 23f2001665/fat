use chrono::NaiveDate;

use crate::utils::calculations::bmi;

/// One immutable snapshot of the user's physical measurements.
///
/// Fields are optional because a user may only have a scale on a given day.
/// Measurements are never overwritten; a new snapshot is appended.
#[derive(Debug, Clone, PartialEq)]
pub struct BodyMeasurement {
    pub id: i64,
    pub user_id: i64,
    pub measured_at: NaiveDate,
    pub weight_kg: Option<f64>,
    pub height_cm: Option<f64>,
    pub body_fat_percentage: Option<f64>,
    pub waist_cm: Option<f64>,
    pub chest_cm: Option<f64>,
    pub hip_cm: Option<f64>,
    pub arm_cm: Option<f64>,
    pub thigh_cm: Option<f64>,
    pub notes: Option<String>,
}

impl BodyMeasurement {
    /// BMI is derived only when both values were recorded in this snapshot.
    pub fn bmi(&self) -> Option<f64> {
        bmi(self.weight_kg, self.height_cm)
    }
}

#[derive(Debug, Clone, Default)]
pub struct NewBodyMeasurement {
    pub measured_at: NaiveDate,
    pub weight_kg: Option<f64>,
    pub height_cm: Option<f64>,
    pub body_fat_percentage: Option<f64>,
    pub waist_cm: Option<f64>,
    pub chest_cm: Option<f64>,
    pub hip_cm: Option<f64>,
    pub arm_cm: Option<f64>,
    pub thigh_cm: Option<f64>,
    pub notes: String,
}

impl NewBodyMeasurement {
    pub fn has_any_value(&self) -> bool {
        self.weight_kg.is_some()
            || self.height_cm.is_some()
            || self.body_fat_percentage.is_some()
            || self.waist_cm.is_some()
            || self.chest_cm.is_some()
            || self.hip_cm.is_some()
            || self.arm_cm.is_some()
            || self.thigh_cm.is_some()
            || !self.notes.trim().is_empty()
    }
}

/// Latest available value for each metric, independently. A measurement that
/// did not include a field does not erase that field's earlier value.
#[derive(Debug, Clone, Default)]
pub struct LatestBodyMetrics {
    pub latest_measurement_date: Option<NaiveDate>,
    pub weight_kg: Option<f64>,
    pub height_cm: Option<f64>,
    pub body_fat_percentage: Option<f64>,
    pub waist_cm: Option<f64>,
    pub chest_cm: Option<f64>,
    pub hip_cm: Option<f64>,
    pub arm_cm: Option<f64>,
    pub thigh_cm: Option<f64>,
    /// Latest non-null BMI, computed from one snapshot.
    pub bmi: Option<f64>,
    pub bmi_measured_at: Option<NaiveDate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MeasurementMetric {
    Weight,
    Bmi,
    BodyFat,
    Waist,
    Chest,
    Arm,
    Thigh,
}

/// Latest vs. previous observed value for one metric. Dates make sparse
/// measurement histories clear instead of implying daily observations.
#[derive(Debug, Clone, PartialEq)]
pub struct MetricComparison {
    pub metric: MeasurementMetric,
    pub current: Option<f64>,
    pub current_date: Option<NaiveDate>,
    pub previous: Option<f64>,
    pub previous_date: Option<NaiveDate>,
}

impl MetricComparison {
    pub fn delta(&self) -> Option<f64> {
        match (self.current, self.previous) {
            (Some(current), Some(previous)) => Some(current - previous),
            _ => None,
        }
    }
}

impl MeasurementMetric {
    pub const ALL: [Self; 7] = [
        Self::Weight,
        Self::Bmi,
        Self::BodyFat,
        Self::Waist,
        Self::Chest,
        Self::Arm,
        Self::Thigh,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Weight => "Body weight",
            Self::Bmi => "BMI",
            Self::BodyFat => "Body-fat %",
            Self::Waist => "Waist",
            Self::Chest => "Chest",
            Self::Arm => "Arm",
            Self::Thigh => "Thigh",
        }
    }

    pub fn unit(self) -> &'static str {
        match self {
            Self::BodyFat => "%",
            Self::Bmi => "",
            _ => "cm/kg",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "weight" => Some(Self::Weight),
            "bmi" => Some(Self::Bmi),
            "body-fat" => Some(Self::BodyFat),
            "waist" => Some(Self::Waist),
            "chest" => Some(Self::Chest),
            "arm" => Some(Self::Arm),
            "thigh" => Some(Self::Thigh),
            _ => None,
        }
    }

    pub fn value(&self, measurement: &BodyMeasurement) -> Option<f64> {
        match self {
            Self::Weight => measurement.weight_kg,
            Self::Bmi => measurement.bmi(),
            Self::BodyFat => measurement.body_fat_percentage,
            Self::Waist => measurement.waist_cm,
            Self::Chest => measurement.chest_cm,
            Self::Arm => measurement.arm_cm,
            Self::Thigh => measurement.thigh_cm,
        }
    }
}
