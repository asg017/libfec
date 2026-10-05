//! Table names: one per `record_family()` value. Pure Rust (no R API), so
//! the mapping is unit-testable with `cargo test`.

/// The R table name of a record family (a `fec_parser::itemizations::record_family`
/// value): `S<x>` families are `schedule_<x>` (`SA` → `schedule_a`, `SA3L` →
/// `schedule_a3l`, `SC1` → `schedule_c1`, `SL` → `schedule_l`, `SI` →
/// `schedule_i`); everything else is lowercased (`H4` → `h4`, `F57` → `f57`,
/// `TEXT` → `text`).
pub fn table_name(family: &str) -> String {
    let lower = family.to_ascii_lowercase();
    let mut chars = lower.chars();
    match (chars.next(), chars.next()) {
        (Some('s'), Some(c)) if c.is_ascii_alphabetic() => format!("schedule_{}", &lower[1..]),
        _ => lower,
    }
}

/// The name of the fallback table of a family that also has typed rows in
/// the same filing (`schedule_a_raw`).
pub fn raw_table_name(family: &str) -> String {
    format!("{}_raw", table_name(family))
}

/// The table of rows `record_family()` doesn't recognize.
pub const OTHER_TABLE: &str = "other";

#[cfg(test)]
mod tests {
    use super::*;
    use fec_parser::itemizations::{record_family, Itemization};

    /// Every value `record_family()` can return: the families with a typed
    /// struct plus `SI` (no struct). `record_family`'s `FAMILIES` is private,
    /// so this list is checked against it below.
    fn all_families() -> Vec<&'static str> {
        let mut v: Vec<&'static str> = Itemization::families().to_vec();
        v.push("SI");
        v
    }

    #[test]
    fn every_family_maps_to_its_documented_name() {
        let expected = [
            ("SA", "schedule_a"),
            ("SB", "schedule_b"),
            ("SD", "schedule_d"),
            ("SF", "schedule_f"),
            ("H1", "h1"),
            ("H2", "h2"),
            ("H3", "h3"),
            ("H4", "h4"),
            ("H5", "h5"),
            ("H6", "h6"),
            ("F56", "f56"),
            ("F57", "f57"),
            ("F65", "f65"),
            ("F76", "f76"),
            ("F91", "f91"),
            ("F92", "f92"),
            ("F93", "f93"),
            ("F94", "f94"),
            ("F132", "f132"),
            ("F133", "f133"),
            ("SL", "schedule_l"),
            ("TEXT", "text"),
            ("SA3L", "schedule_a3l"),
            ("SE", "schedule_e"),
            ("SC", "schedule_c"),
            ("SC1", "schedule_c1"),
            ("SC2", "schedule_c2"),
            ("SI", "schedule_i"),
        ];
        let families = all_families();
        assert_eq!(families.len(), expected.len());
        for (family, name) in expected {
            assert!(families.contains(&family), "{family} not a family");
            assert_eq!(table_name(family), name, "{family}");
        }
    }

    #[test]
    fn families_are_record_family_values_and_names_are_unique_and_plain() {
        let families = all_families();
        let mut names = std::collections::HashSet::new();
        for family in &families {
            // Each is a fixed point of record_family, i.e. in its FAMILIES.
            assert_eq!(record_family(family), Some(*family));
            let name = table_name(family);
            assert!(
                name.bytes()
                    .all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'_'),
                "{name}"
            );
            assert!(name != OTHER_TABLE);
            assert!(names.insert(name.clone()), "duplicate {name}");
            assert!(!names.contains(&raw_table_name(family)));
        }
        // Row types map through their family, never their own spelling.
        assert_eq!(
            record_family("SL/10").map(table_name).as_deref(),
            Some("schedule_l")
        );
        assert_eq!(
            record_family("SC1/10").map(table_name).as_deref(),
            Some("schedule_c1")
        );
        assert_eq!(
            record_family("SA3L").map(table_name).as_deref(),
            Some("schedule_a3l")
        );
        assert_eq!(
            record_family("sa11ai").map(table_name).as_deref(),
            Some("schedule_a")
        );
        assert_eq!(record_family("ZZZ"), None);
    }
}
