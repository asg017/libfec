use crate::mappings;

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub enum ScheduleType {
    // Itemized Receipts
    // https://github.com/fecgov/fecfile-validate/blob/develop/schema/backlog/schedules/SchA.json
    ScheduleA,
    // Itemized Disbursements
    // https://github.com/fecgov/fecfile-validate/blob/develop/schema/backlog/schedules/SchB.json
    ScheduleB,
    // Loans
    // https://github.com/fecgov/fecfile-validate/blob/develop/schema/backlog/schedules/SchC.json
    ScheduleC,

    // Loans And Lines Of Credit From Lending Institutions
    // https://github.com/fecgov/fecfile-validate/blob/develop/schema/backlog/schedules/SchC1.json
    ScheduleC1,

    // Loan Guarantor Name & Address Information
    // https://github.com/fecgov/fecfile-validate/blob/develop/schema/backlog/schedules/SchC2.json
    ScheduleC2,

    // DEBTS AND OBLIGATIONS  (Itemized for each one)
    // https://github.com/fecgov/fecfile-validate/blob/develop/schema/backlog/schedules/SchD.json
    ScheduleD,

    // ITEMIZED INDEPENDENT EXPENDITURES
    // https://github.com/fecgov/fecfile-validate/blob/develop/schema/backlog/schedules/SchE.json
    ScheduleE,

    // ITEMIZED COORDINATED EXPENDITURES MADE BY POLITICAL PARTY COMMITTEES OR DESIGNATED AGENT(S) ON BEHALF OF CANDIDATES FOR FEDERAL OFFICE
    // https://github.com/fecgov/fecfile-validate/blob/develop/schema/backlog/schedules/SchF.json
    ScheduleF,
    //ScheduleH1,
    //ScheduleH2,
    //ScheduleH3,
    //ScheduleH4,
    //ScheduleH5,
    //ScheduleH6,
    //ScheduleI,
}

pub fn form_type_schedule_type(form_type: &str) -> Option<ScheduleType> {
    if form_type.starts_with("SA") {
        Some(ScheduleType::ScheduleA)
    } else if form_type.starts_with("SB") {
        Some(ScheduleType::ScheduleB)
    } else if form_type.starts_with("SC1") {
        Some(ScheduleType::ScheduleC1)
    } else if form_type.starts_with("SC2") {
        Some(ScheduleType::ScheduleC2)
    } else if form_type.starts_with("SC") {
        Some(ScheduleType::ScheduleC)
    } else if form_type.starts_with("SD") {
        Some(ScheduleType::ScheduleD)
    } else if form_type.starts_with("SE") {
        Some(ScheduleType::ScheduleE)
    } else if form_type.starts_with("SF") {
        Some(ScheduleType::ScheduleF)
    } else {
        None
    }
}

impl ScheduleType {
    /// A row type of the schedule's ordinary itemizations, whose layout is
    /// the schedule's: `SA11AI`, `SB21B`, `SC/10`, ... Variant row types
    /// such as `SA3L` (lobbyist bundling, a different 8.x layout) or paper's
    /// `SASL` are filed under the schedule too but don't define it.
    pub fn canonical_row_type(&self) -> &'static str {
        match self {
            ScheduleType::ScheduleA => "SA11AI",
            ScheduleType::ScheduleB => "SB21B",
            ScheduleType::ScheduleC => "SC/10",
            ScheduleType::ScheduleC1 => "SC1/10",
            ScheduleType::ScheduleC2 => "SC2/10",
            ScheduleType::ScheduleD => "SD10",
            ScheduleType::ScheduleE => "SE",
            ScheduleType::ScheduleF => "SF",
        }
    }

    /// The columns of the schedule's ordinary itemizations (see
    /// [`ScheduleType::canonical_row_type`]) in `fec_version`. Exports use
    /// the `8.5` layout for every row of a schedule, whatever its row type
    /// and version, so the output doesn't depend on which row comes first.
    pub fn column_names(&self, fec_version: &str) -> anyhow::Result<Vec<String>> {
        Ok(mappings::column_names_for_field(self.canonical_row_type(), fec_version)?.to_owned())
    }
}

impl std::fmt::Display for ScheduleType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScheduleType::ScheduleA => write!(f, "SA"),
            ScheduleType::ScheduleB => write!(f, "SB"),
            ScheduleType::ScheduleC => write!(f, "SC"),
            ScheduleType::ScheduleC1 => write!(f, "SC1"),
            ScheduleType::ScheduleC2 => write!(f, "SC2"),
            ScheduleType::ScheduleD => write!(f, "SD"),
            ScheduleType::ScheduleE => write!(f, "SE"),
            ScheduleType::ScheduleF => write!(f, "SF"),
        }
    }
}

impl ScheduleType {
    pub fn to_sqlite_tablename(self) -> String {
        match self {
            ScheduleType::ScheduleA => "schedule_a".to_string(),
            ScheduleType::ScheduleB => "schedule_b".to_string(),
            ScheduleType::ScheduleC => "schedule_c".to_string(),
            ScheduleType::ScheduleC1 => "schedule_c1".to_string(),
            ScheduleType::ScheduleC2 => "schedule_c2".to_string(),
            ScheduleType::ScheduleD => "schedule_d".to_string(),
            ScheduleType::ScheduleE => "schedule_e".to_string(),
            ScheduleType::ScheduleF => "schedule_f".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_schedule_has_an_8_5_layout_of_its_own_type() {
        for schedule in [
            ScheduleType::ScheduleA,
            ScheduleType::ScheduleB,
            ScheduleType::ScheduleC,
            ScheduleType::ScheduleC1,
            ScheduleType::ScheduleC2,
            ScheduleType::ScheduleD,
            ScheduleType::ScheduleE,
            ScheduleType::ScheduleF,
        ] {
            let row_type = schedule.canonical_row_type();
            assert_eq!(form_type_schedule_type(row_type), Some(schedule));
            let columns = schedule.column_names("8.5").expect("8.5 layout");
            assert_eq!(columns[0], "form_type", "{schedule}");
        }
        // Not the lobbyist-bundling layout of SA3L.
        let sa = ScheduleType::ScheduleA.column_names("8.5").expect("SA");
        assert!(sa.iter().any(|c| c == "contributor_last_name"));
    }
}
