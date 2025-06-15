use std::sync::LazyLock;


#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

static SCHED_A: LazyLock<Vec<String>> = LazyLock::new(|| {
  crate::mappings::column_names_for_field("SA", "8.4").unwrap().to_owned()
});

impl ToString for ScheduleType {
    fn to_string(&self) -> String {
        match self {
            ScheduleType::ScheduleA => "SA".to_string(),
            ScheduleType::ScheduleB => "SB".to_string(),
            ScheduleType::ScheduleC => "SC".to_string(),
            ScheduleType::ScheduleC1 => "SC1".to_string(),
            ScheduleType::ScheduleC2 => "SC2".to_string(),
            ScheduleType::ScheduleD => "SD".to_string(),
            ScheduleType::ScheduleE => "SE".to_string(),
            ScheduleType::ScheduleF => "SF".to_string(),
        }
    }
}