use ui_payload::DiffPreset;

pub fn preset_from_selection(selection: i32) -> DiffPreset {
    match selection {
        1 => DiffPreset::Fastest,
        2 => DiffPreset::MostPrecise,
        _ => DiffPreset::Balanced,
    }
}

pub fn parse_globs(text: &str) -> Option<Vec<String>> {
    let items: Vec<String> = text
        .split([',', '\n'])
        .map(|entry| entry.trim())
        .filter(|entry| !entry.is_empty())
        .map(|entry| entry.to_string())
        .collect();
    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

pub fn base_name(path: &str) -> String {
    let parts: Vec<&str> = path.split(['\\', '/']).collect();
    parts.last().unwrap_or(&path).to_string()
}

#[cfg(test)]
mod tests {
    use super::{base_name, parse_globs, preset_from_selection};
    use ui_payload::DiffPreset;

    #[test]
    fn preset_from_selection_maps_choices() {
        assert_eq!(preset_from_selection(0), DiffPreset::Balanced);
        assert_eq!(preset_from_selection(1), DiffPreset::Fastest);
        assert_eq!(preset_from_selection(2), DiffPreset::MostPrecise);
        assert_eq!(preset_from_selection(9), DiffPreset::Balanced);
    }

    #[test]
    fn parse_globs_splits_and_trims() {
        let result = parse_globs(" foo,bar \n baz ");
        assert_eq!(
            result,
            Some(vec!["foo".to_string(), "bar".to_string(), "baz".to_string()])
        );
        assert!(parse_globs(" \n , ").is_none());
    }

    #[test]
    fn base_name_handles_paths() {
        assert_eq!(base_name("report.xlsx"), "report.xlsx");
        assert_eq!(base_name("/tmp/report.xlsx"), "report.xlsx");
        assert_eq!(base_name("C:\\tmp\\report.xlsx"), "report.xlsx");
    }
}
