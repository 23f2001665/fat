use crate::error::{AppError, AppResult};

/// Effective repetitions: complete reps plus half-reps.
///
/// `partial_reps` counts half repetitions, so 2 partial reps equal one full rep.
pub fn effective_reps(complete_reps: i64, partial_reps: i64) -> f64 {
    complete_reps as f64 + partial_reps as f64 / 2.0
}

/// Training volume in kilograms.
pub fn set_volume(weight_kg: f64, complete_reps: i64, partial_reps: i64) -> f64 {
    weight_kg * effective_reps(complete_reps, partial_reps)
}

/// BMI from a weight/height pair. Height is stored in centimetres.
///
/// BMI is a screening metric, not a body-composition measurement. Both values
/// must come from the same measurement snapshot to avoid mixing observations.
pub fn bmi(weight_kg: Option<f64>, height_cm: Option<f64>) -> Option<f64> {
    let weight = weight_kg?;
    let height = height_cm?;
    if weight <= 0.0 || height <= 0.0 {
        return None;
    }
    let height_m = height / 100.0;
    Some(weight / (height_m * height_m))
}

pub fn bmi_category(bmi: Option<f64>) -> &'static str {
    match bmi {
        None => "Unavailable",
        Some(value) if value < 18.5 => "Underweight",
        Some(value) if value < 25.0 => "Healthy range",
        Some(value) if value < 30.0 => "Overweight",
        Some(_) => "Obese range",
    }
}

pub fn avg(values: impl Iterator<Item = f64>) -> Option<f64> {
    let mut sum = 0.0;
    let mut count = 0usize;
    for value in values {
        sum += value;
        count += 1;
    }
    (count > 0).then_some(sum / count as f64)
}

pub fn validate_workout_values(
    weight_kg: f64,
    complete_reps: i64,
    partial_reps: i64,
    rest_seconds: Option<i64>,
    rir: Option<i64>,
) -> AppResult<()> {
    if !weight_kg.is_finite() || weight_kg < 0.0 {
        return Err(AppError::validation(
            "Weight must be a finite, non-negative number",
        ));
    }
    if weight_kg > 2_000.0 {
        return Err(AppError::validation(
            "Weight looks unreasonable (max 2000 kg)",
        ));
    }
    if !(0..=10_000).contains(&complete_reps) {
        return Err(AppError::validation(
            "Complete reps must be between 0 and 10000",
        ));
    }
    if !(0..=10_000).contains(&partial_reps) {
        return Err(AppError::validation(
            "Partial reps must be between 0 and 10000",
        ));
    }
    if complete_reps == 0 && partial_reps == 0 {
        return Err(AppError::validation(
            "A set must have at least one complete or partial rep",
        ));
    }
    if let Some(rest) = rest_seconds {
        if !(0..=86_400).contains(&rest) {
            return Err(AppError::validation(
                "Rest duration must be between 0 and 86400 seconds",
            ));
        }
    }
    if let Some(rir) = rir {
        if !(0..=10).contains(&rir) {
            return Err(AppError::validation("RIR must be between 0 and 10"));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partial_reps_are_half_reps() {
        assert_eq!(effective_reps(8, 0), 8.0);
        assert_eq!(effective_reps(8, 1), 8.5);
        assert_eq!(effective_reps(8, 2), 9.0);
        assert_eq!(effective_reps(8, 3), 9.5);
    }

    #[test]
    fn volume_uses_effective_repetitions() {
        assert_eq!(set_volume(30.0, 6, 0), 180.0);
        assert_eq!(set_volume(30.0, 6, 1), 195.0);
    }

    #[test]
    fn bmi_requires_both_values() {
        assert_eq!(bmi(Some(80.0), Some(180.0)), Some(24.691358024691358));
        assert_eq!(bmi(None, Some(180.0)), None);
        assert_eq!(bmi(Some(80.0), None), None);
        assert_eq!(bmi(Some(0.0), Some(180.0)), None);
    }

    #[test]
    fn rir_must_be_in_range() {
        assert!(validate_workout_values(20.0, 5, 0, None, Some(11)).is_err());
        assert!(validate_workout_values(20.0, 5, 0, None, Some(0)).is_ok());
    }

    #[test]
    fn empty_sets_are_rejected() {
        assert!(validate_workout_values(0.0, 0, 0, None, None).is_err());
    }
}
