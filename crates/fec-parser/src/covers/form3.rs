use crate::covers::Treasurer;
use indexmap::IndexMap;
use jiff::civil::Date;

/// "FORM 3 - Report Of Receipts And Disbursements For An Authorized Committee"
pub struct Form3 {
    pub treasurer: Treasurer,
    pub signed: Date,
    pub summary: Form3Summary,
    pub detailed_summary: Form3DetailedSummary,
}

pub struct Form3Summary {
    pub line6_total_contributions_no_loans: f64,
    pub line7_total_contribution_refunds: f64,
    pub line8_net_contributions: f64,
    pub line9_total_operating_expenditures: f64,
    pub line10_total_offset_to_operating_expenditures: f64,
    pub line11_net_operating_expenditures: f64,
    pub line12_cash_on_hand_close_of_period: f64,
    pub line13_debts_owed_to_committee: f64,
    pub line14_debts_owed_by_committee: f64,
}

pub struct DetailedSummaryRow {
    pub column_a: f64,
    pub column_b: f64,
}

impl DetailedSummaryRow {
    pub fn from_data(
        data: &IndexMap<String, String>,
        column_a_key: &str,
        column_b_key: &str,
    ) -> Self {
        Self {
            column_a: data
                .get(column_a_key)
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            column_b: data
                .get(column_b_key)
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
        }
    }
}

/*
Form 3 columns:
      "col_a_total_contributions_no_loans",
      "col_a_total_contributions_refunds",
      "col_a_net_contributions",
      "col_a_total_operating_expenditures",
      "col_a_total_offset_to_operating_expenditures",
      "col_a_net_operating_expenditures",
      "col_a_cash_on_hand_close_of_period",
      "col_a_debts_to",
      "col_a_debts_by",
      "col_a_individual_contributions_itemized",
      "col_a_individual_contributions_unitemized",
      "col_a_total_individual_contributions",
      "col_a_political_party_contributions",
      "col_a_pac_contributions",
      "col_a_candidate_contributions",
      "col_a_total_contributions",
      "col_a_transfers_from_authorized",
      "col_a_candidate_loans",
      "col_a_other_loans",
      "col_a_total_loans",
      "col_a_offset_to_operating_expenditures",
      "col_a_other_receipts",
      "col_a_total_receipts",
      "col_a_operating_expenditures",
      "col_a_transfers_to_authorized",
      "col_a_candidate_loan_repayments",
      "col_a_other_loan_repayments",
      "col_a_total_loan_repayments",
      "col_a_refunds_to_individuals",
      "col_a_refunds_to_party_committees",
      "col_a_refunds_to_other_committees",
      "col_a_total_refunds",
      "col_a_other_disbursements",
      "col_a_total_disbursements",
      "col_a_cash_beginning_reporting_period",
      "col_a_total_receipts_period",
      "col_a_subtotals",
      "col_a_total_disbursements_period",
      "col_a_cash_on_hand_close",
      "col_b_total_contributions_no_loans",
      "col_b_total_contributions_refunds",
      "col_b_net_contributions",
      "col_b_total_operating_expenditures",
      "col_b_total_offset_to_operating_expenditures",
      "col_b_net_operating_expenditures",
      "col_b_individual_contributions_itemized",
      "col_b_individual_contributions_unitemized",
      "col_b_total_individual_contributions",
      "col_b_political_party_contributions",
      "col_b_pac_contributions",
      "col_b_candidate_contributions",
      "col_b_total_contributions",
      "col_b_transfers_from_authorized",
      "col_b_candidate_loans",
      "col_b_other_loans",
      "col_b_total_loans",
      "col_b_offset_to_operating_expenditures",
      "col_b_other_receipts",
      "col_b_total_receipts",
      "col_b_operating_expenditures",
      "col_b_transfers_to_authorized",
      "col_b_candidate_loan_repayments",
      "col_b_other_loan_repayments",
      "col_b_total_loan_repayments",
      "col_b_refunds_to_individuals",
      "col_b_refunds_to_party_committees",
      "col_b_refunds_to_other_committees",
      "col_b_total_refunds",
      "col_b_other_disbursements",
      "col_b_total_disbursements"
*/

pub struct Form3DetailedSummaryReceipts {
    pub line11a_i_contributions_from_individuals_itemized: DetailedSummaryRow,
    pub line11a_ii_contributions_from_individuals_unitemized: DetailedSummaryRow,
    pub line11a_iii_contributions_from_individuals_total: DetailedSummaryRow,
    pub line11b_political_party_committees: DetailedSummaryRow,
    pub line11c_other_political_committees_pacs: DetailedSummaryRow,
    pub line11d_the_candidate: DetailedSummaryRow,
    pub line11e_total_contributions: DetailedSummaryRow,
    pub line12_transfers_from_authorized: DetailedSummaryRow,
    pub line13a_loans_from_candidate: DetailedSummaryRow,
    pub line13b_other_loans: DetailedSummaryRow,
    pub line13c_total_loans: DetailedSummaryRow,
    pub line14_offset_to_operating_expenditures: DetailedSummaryRow,
    pub line15_other_receipts: DetailedSummaryRow,
    pub line16_total_receipts: DetailedSummaryRow,
}

impl Form3DetailedSummaryReceipts {
    pub fn from_data(data: &IndexMap<String, String>) -> Self {
        Self {
            line11a_i_contributions_from_individuals_itemized: DetailedSummaryRow::from_data(
                data,
                "col_a_individual_contributions_itemized",
                "col_b_individual_contributions_itemized",
            ),
            line11a_ii_contributions_from_individuals_unitemized: DetailedSummaryRow::from_data(
                data,
                "col_a_individual_contributions_unitemized",
                "col_b_individual_contributions_unitemized",
            ),
            line11a_iii_contributions_from_individuals_total: DetailedSummaryRow::from_data(
                data,
                "col_a_total_individual_contributions",
                "col_b_total_individual_contributions",
            ),
            line11b_political_party_committees: DetailedSummaryRow::from_data(
                data,
                "col_a_political_party_contributions",
                "col_b_political_party_contributions",
            ),
            line11c_other_political_committees_pacs: DetailedSummaryRow::from_data(
                data,
                "col_a_pac_contributions",
                "col_b_pac_contributions",
            ),
            line11d_the_candidate: DetailedSummaryRow::from_data(
                data,
                "col_a_candidate_contributions",
                "col_b_candidate_contributions",
            ),
            line11e_total_contributions: DetailedSummaryRow::from_data(
                data,
                "col_a_total_contributions",
                "col_b_total_contributions",
            ),
            line12_transfers_from_authorized: DetailedSummaryRow::from_data(
                data,
                "col_a_transfers_from_authorized",
                "col_b_transfers_from_authorized",
            ),
            line13a_loans_from_candidate: DetailedSummaryRow::from_data(
                data,
                "col_a_candidate_loans",
                "col_b_candidate_loans",
            ),
            line13b_other_loans: DetailedSummaryRow::from_data(
                data,
                "col_a_other_loans",
                "col_b_other_loans",
            ),
            line13c_total_loans: DetailedSummaryRow::from_data(
                data,
                "col_a_total_loans",
                "col_b_total_loans",
            ),
            line14_offset_to_operating_expenditures: DetailedSummaryRow::from_data(
                data,
                "col_a_offset_to_operating_expenditures",
                "col_b_offset_to_operating_expenditures",
            ),
            line15_other_receipts: DetailedSummaryRow::from_data(
                data,
                "col_a_other_receipts",
                "col_b_other_receipts",
            ),
            line16_total_receipts: DetailedSummaryRow::from_data(
                data,
                "col_a_total_receipts",
                "col_b_total_receipts",
            ),
        }
    }
}

pub struct Form3DetailedSummaryDisbursements {
    pub line_17_operating_expenditures: DetailedSummaryRow,
    pub line_18_transfers_to_authorized: DetailedSummaryRow,
    pub line_19a_candidate_loan_repayments: DetailedSummaryRow,
    pub line_19b_other_loan_repayments: DetailedSummaryRow,
    pub line_19c_total_loan_repayments: DetailedSummaryRow,
    pub line_20a_refunds_to_individuals: DetailedSummaryRow,
    pub line_20b_refunds_to_party_committees: DetailedSummaryRow,
    pub line_20c_refunds_to_other_committees: DetailedSummaryRow,
    pub line_20d_total_refunds: DetailedSummaryRow,
    pub line_21_other_disbursements: DetailedSummaryRow,
    pub line_22_total_disbursements: DetailedSummaryRow,
}

impl Form3DetailedSummaryDisbursements {
    pub fn from_data(data: &IndexMap<String, String>) -> Self {
        Self {
            line_17_operating_expenditures: DetailedSummaryRow::from_data(
                data,
                "col_a_operating_expenditures",
                "col_b_operating_expenditures",
            ),
            line_18_transfers_to_authorized: DetailedSummaryRow::from_data(
                data,
                "col_a_transfers_to_authorized",
                "col_b_transfers_to_authorized",
            ),
            line_19a_candidate_loan_repayments: DetailedSummaryRow::from_data(
                data,
                "col_a_candidate_loan_repayments",
                "col_b_candidate_loan_repayments",
            ),
            line_19b_other_loan_repayments: DetailedSummaryRow::from_data(
                data,
                "col_a_other_loan_repayments",
                "col_b_other_loan_repayments",
            ),
            line_19c_total_loan_repayments: DetailedSummaryRow::from_data(
                data,
                "col_a_total_loan_repayments",
                "col_b_total_loan_repayments",
            ),
            line_20a_refunds_to_individuals: DetailedSummaryRow::from_data(
                data,
                "col_a_refunds_to_individuals",
                "col_b_refunds_to_individuals",
            ),
            line_20b_refunds_to_party_committees: DetailedSummaryRow::from_data(
                data,
                "col_a_refunds_to_party_committees",
                "col_b_refunds_to_party_committees",
            ),
            line_20c_refunds_to_other_committees: DetailedSummaryRow::from_data(
                data,
                "col_a_refunds_to_other_committees",
                "col_b_refunds_to_other_committees",
            ),
            line_20d_total_refunds: DetailedSummaryRow::from_data(
                data,
                "col_a_total_refunds",
                "col_b_total_refunds",
            ),
            line_21_other_disbursements: DetailedSummaryRow::from_data(
                data,
                "col_a_other_disbursements",
                "col_b_other_disbursements",
            ),
            line_22_total_disbursements: DetailedSummaryRow::from_data(
                data,
                "col_a_total_disbursements",
                "col_b_total_disbursements",
            ),
        }
    }
}

pub struct Form3DetailedSummary {
    pub receipts: Form3DetailedSummaryReceipts,
    pub disbursements: Form3DetailedSummaryDisbursements,
    pub cash_on_hand_beginning: f64,
    pub total_receipts_period: f64,
    pub subtotals: f64,
    pub total_disbursements_period: f64,
    pub cash_on_hand_close: f64,
}

impl Form3DetailedSummary {
    pub fn from_data(data: &IndexMap<String, String>) -> Self {
        Self {
            receipts: Form3DetailedSummaryReceipts::from_data(data),
            disbursements: Form3DetailedSummaryDisbursements::from_data(data),
            cash_on_hand_beginning: data
                .get("col_a_cash_beginning_reporting_period")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            total_receipts_period: data
                .get("col_a_total_receipts_period")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            subtotals: data
                .get("col_a_subtotals")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            total_disbursements_period: data
                .get("col_a_total_disbursements_period")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
            cash_on_hand_close: data
                .get("col_a_cash_on_hand_close")
                .and_then(|s| s.parse().ok())
                .unwrap_or(0.0),
        }
    }
}

impl Form3 {
    pub fn from_data(data: &IndexMap<String, String>) -> Option<Self> {
        let signed = data
            .get("date_signed")
            .and_then(|s| Date::strptime("%Y%m%d", s).ok())?;

        let summary = Form3Summary {
            line6_total_contributions_no_loans: data
                .get("col_a_total_contributions_no_loans")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line7_total_contribution_refunds: data
                .get("col_a_total_contributions_refunds")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line8_net_contributions: data
                .get("col_a_net_contributions")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line9_total_operating_expenditures: data
                .get("col_a_total_operating_expenditures")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line10_total_offset_to_operating_expenditures: data
                .get("col_a_total_offset_to_operating_expenditures")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line11_net_operating_expenditures: data
                .get("col_a_net_operating_expenditures")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line12_cash_on_hand_close_of_period: data
                .get("col_a_cash_on_hand_close_of_period")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line13_debts_owed_to_committee: data
                .get("col_a_debts_to")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line14_debts_owed_by_committee: data
                .get("col_a_debts_by")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
        };

        Some(Self {
            treasurer: Treasurer::from_data(data),
            signed,
            summary,
            detailed_summary: Form3DetailedSummary::from_data(data),
        })
    }
}
