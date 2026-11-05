//! Presentation helpers. All calculations live in the domain layer; this
//! module only turns already-computed values into user-facing strings.

pub fn number(value: f64, decimals: usize) -> String {
    if !value.is_finite() {
        return "-".into();
    }
    let mut text = format!("{value:.decimals$}");
    if text.contains('.') {
        while text.ends_with('0') {
            text.pop();
        }
        if text.ends_with('.') {
            text.pop();
        }
    }
    if text == "-0" {
        text = "0".into();
    }
    text
}

pub fn grouped(value: f64, decimals: usize) -> String {
    let text = number(value, decimals);
    let (sign, text) = if let Some(rest) = text.strip_prefix('-') {
        ("-", rest)
    } else {
        ("", text.as_str())
    };
    let (integer, fraction) = match text.split_once('.') {
        Some((integer, fraction)) => (integer, Some(fraction)),
        None => (text, None),
    };
    let mut grouped = String::new();
    for (index, ch) in integer.chars().rev().enumerate() {
        if index > 0 && index % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    let mut result: String = grouped.chars().rev().collect();
    result.insert_str(0, sign);
    if let Some(fraction) = fraction {
        result.push('.');
        result.push_str(fraction);
    }
    result
}

pub fn kg(value: f64) -> String {
    format!("{} kg", grouped(value, 1))
}

pub fn optional_kg(value: Option<f64>) -> String {
    value.map(kg).unwrap_or_else(|| "-".into())
}

pub fn reps(complete: i64, partial: i64) -> String {
    if partial > 0 {
        format!("{complete} + {partial} partial")
    } else {
        complete.to_string()
    }
}

pub fn bytes(value: u64) -> String {
    const UNITS: [&str; 4] = ["B", "KB", "MB", "GB"];
    let mut size = value as f64;
    let mut unit = 0;
    while size >= 1024.0 && unit < UNITS.len() - 1 {
        size /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{value} B")
    } else {
        format!("{} {}", number(size, 1), UNITS[unit])
    }
}

pub fn signed(value: f64, decimals: usize, unit: &str) -> String {
    let sign = if value > 0.0 { "+" } else { "" };
    let text = format!("{sign}{}", number(value, decimals));
    if unit.is_empty() {
        text
    } else {
        format!("{text} {unit}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_trim_trailing_zeroes() {
        assert_eq!(number(12.0, 1), "12");
        assert_eq!(number(12.5, 2), "12.5");
        assert_eq!(number(12.26, 1), "12.3");
    }

    #[test]
    fn grouping_keeps_decimals() {
        assert_eq!(grouped(1234567.0, 0), "1,234,567");
        assert_eq!(grouped(1234.5, 1), "1,234.5");
        assert_eq!(grouped(-1234.0, 0), "-1,234");
    }

    #[test]
    fn bytes_use_readable_units() {
        assert_eq!(bytes(512), "512 B");
        assert_eq!(bytes(2048), "2 KB");
    }
}
