use crate::covers::Treasurer;
use indexmap::IndexMap;
use jiff::civil::Date;

/// "FORM 1 - Statement of Organization"
/// Filed by committees to register with the FEC or to update their registration info.
pub struct Form1 {
    pub committee_name: String,
    pub street_1: String,
    pub street_2: Option<String>,
    pub city: String,
    pub state: String,
    pub zip_code: String,
    pub committee_email: Option<String>,
    pub committee_url: Option<String>,
    pub effective_date: Option<Date>,
    pub date_signed: Option<Date>,
    pub committee_type: Option<String>,
    pub candidate: Option<Form1Candidate>,
    pub party_code: Option<String>,
    pub party_type: Option<String>,
    pub organization_type: Option<String>,
    pub leadership_pac: Option<String>,
    pub treasurer: Treasurer,
}

pub struct Form1Candidate {
    pub candidate_id: String,
    pub last_name: String,
    pub first_name: String,
    pub middle_name: Option<String>,
    pub prefix: Option<String>,
    pub suffix: Option<String>,
    pub office: Option<String>,
    pub state: Option<String>,
    pub district: Option<String>,
}

impl Form1Candidate {
    pub fn from_data(data: &IndexMap<String, String>) -> Option<Self> {
        let candidate_id = data.get("candidate_id_number")?.clone();
        if candidate_id.is_empty() {
            return None;
        }
        Some(Self {
            candidate_id,
            last_name: data.get("candidate_last_name").cloned().unwrap_or_default(),
            first_name: data.get("candidate_first_name").cloned().unwrap_or_default(),
            middle_name: data.get("candidate_middle_name").cloned().filter(|s| !s.is_empty()),
            prefix: data.get("candidate_prefix").cloned().filter(|s| !s.is_empty()),
            suffix: data.get("candidate_suffix").cloned().filter(|s| !s.is_empty()),
            office: data.get("candidate_office").cloned().filter(|s| !s.is_empty()),
            state: data.get("candidate_state").cloned().filter(|s| !s.is_empty()),
            district: data.get("candidate_district").cloned().filter(|s| !s.is_empty()),
        })
    }

    pub fn full_name(&self) -> String {
        let mut name = String::new();
        if let Some(prefix) = &self.prefix {
            name.push_str(prefix.trim());
            name.push(' ');
        }
        name.push_str(&self.first_name.trim());
        if let Some(middle_name) = &self.middle_name {
            name.push(' ');
            name.push_str(middle_name.trim());
        }
        name.push(' ');
        name.push_str(&self.last_name.trim());
        if let Some(suffix) = &self.suffix {
            name.push(' ');
            name.push_str(suffix.trim());
        }
        name.trim().to_string()
    }
}

impl Form1 {
    pub fn from_data(data: &IndexMap<String, String>) -> Option<Self> {
        let committee_name = data.get("committee_name")?.clone();

        Some(Self {
            committee_name,
            street_1: data.get("street_1").cloned().unwrap_or_default(),
            street_2: data.get("street_2").cloned().filter(|s| !s.is_empty()),
            city: data.get("city").cloned().unwrap_or_default(),
            state: data.get("state").cloned().unwrap_or_default(),
            zip_code: data.get("zip_code").cloned().unwrap_or_default(),
            committee_email: data.get("committee_email").cloned().filter(|s| !s.is_empty()),
            committee_url: data.get("committee_url").cloned().filter(|s| !s.is_empty()),
            effective_date: data
                .get("effective_date")
                .and_then(|s| Date::strptime("%Y%m%d", s).ok()),
            date_signed: data
                .get("date_signed")
                .and_then(|s| Date::strptime("%Y%m%d", s).ok()),
            committee_type: data.get("committee_type").cloned().filter(|s| !s.is_empty()),
            candidate: Form1Candidate::from_data(data),
            party_code: data.get("party_code").cloned().filter(|s| !s.is_empty()),
            party_type: data.get("party_type").cloned().filter(|s| !s.is_empty()),
            organization_type: data.get("organization_type").cloned().filter(|s| !s.is_empty()),
            leadership_pac: data.get("leadership_pac").cloned().filter(|s| !s.is_empty()),
            treasurer: Treasurer::from_data(data),
        })
    }
}
