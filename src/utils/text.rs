use crate::error::{AppError, AppResult};

/// Normalizes a user-entered reference name for duplicate detection.
///
/// Leading/trailing whitespace and repeated internal whitespace are collapsed,
/// and the comparison is case-insensitive.
pub fn normalize_name(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// Trims and collapses whitespace, preserving the user's capitalization.
pub fn clean_name(value: &str) -> String {
    value.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn required_name(value: &str, field: &str) -> AppResult<String> {
    let cleaned = clean_name(value);
    if cleaned.is_empty() {
        return Err(AppError::validation(format!("{field} cannot be empty")));
    }
    if cleaned.chars().count() > 120 {
        return Err(AppError::validation(format!(
            "{field} cannot be longer than 120 characters"
        )));
    }
    Ok(cleaned)
}

pub fn optional_text(value: &str, field: &str) -> AppResult<Option<String>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        Ok(None)
    } else if trimmed.chars().count() > 4_000 {
        Err(AppError::validation(format!(
            "{field} cannot be longer than 4000 characters"
        )))
    } else {
        Ok(Some(trimmed.to_string()))
    }
}

/// Parses an optional decimal field. Empty input means "not provided".
pub fn optional_decimal(value: &str, field: &str) -> AppResult<Option<f64>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed: f64 = trimmed
        .parse()
        .map_err(|_| AppError::validation(format!("{field} must be a number")))?;
    if !parsed.is_finite() {
        return Err(AppError::validation(format!(
            "{field} must be a finite number"
        )));
    }
    Ok(Some(parsed))
}

/// Parses an optional non-negative integer. Empty input means "not provided".
pub fn optional_whole_number(value: &str, field: &str) -> AppResult<Option<i64>> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Ok(None);
    }
    let parsed: i64 = trimmed
        .parse()
        .map_err(|_| AppError::validation(format!("{field} must be a whole number")))?;
    Ok(Some(parsed))
}

pub fn required_decimal(value: &str, field: &str) -> AppResult<f64> {
    optional_decimal(value, field)?
        .ok_or_else(|| AppError::validation(format!("{field} is required")))
}

pub fn required_whole_number(value: &str, field: &str, default: i64) -> AppResult<i64> {
    Ok(optional_whole_number(value, field)?.unwrap_or(default))
}

pub fn validate_non_negative(value: Option<f64>, field: &str) -> AppResult<()> {
    if let Some(value) = value {
        if value < 0.0 {
            return Err(AppError::validation(format!("{field} cannot be negative")));
        }
    }
    Ok(())
}

pub fn validate_positive(value: Option<f64>, field: &str) -> AppResult<()> {
    if let Some(value) = value {
        if value <= 0.0 {
            return Err(AppError::validation(format!(
                "{field} must be greater than zero"
            )));
        }
    }
    Ok(())
}

pub fn validate_range(value: Option<f64>, min: f64, max: f64, field: &str) -> AppResult<()> {
    if let Some(value) = value {
        if !(min..=max).contains(&value) {
            return Err(AppError::validation(format!(
                "{field} must be between {min} and {max}"
            )));
        }
    }
    Ok(())
}

/// Formats a number for display without trailing zeros.
pub fn format_number(value: f64, decimals: usize) -> String {
    let rounded = format!("{value:.decimals$}");
    let trimmed = rounded.trim_end_matches('0').trim_end_matches('.');
    if trimmed.is_empty() {
        "0".to_string()
    } else {
        trimmed.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalization_ignores_case_and_spacing() {
        assert_eq!(
            normalize_name("  Barbell   BENCH Press "),
            "barbell bench press"
        );
    }

    #[test]
    fn empty_names_are_rejected() {
        assert!(required_name("   ", "Name").is_err());
    }

    #[test]
    fn decimal_parsing_rejects_nan() {
        assert!(optional_decimal("NaN", "Weight").is_err());
        assert_eq!(optional_decimal("  ", "Weight").unwrap(), None);
    }

    #[test]
    fn number_formatting_is_compact() {
        assert_eq!(format_number(62.5, 2), "62.5");
        assert_eq!(format_number(100.0, 1), "100");
    }
}
