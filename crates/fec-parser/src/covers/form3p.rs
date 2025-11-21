use crate::covers::Treasurer;
use indexmap::IndexMap;
use jiff::civil::Date;

/// "FORM 3P - Report Of Receipts And Disbursements By An Authorized Committee Of A Candidate For The Office Of President Or Vice-President"
pub struct Form3P {
    pub treasurer: Treasurer,
    pub signed: Date,
    pub summary: Form3PSummary,
    pub detailed_summary: Form3PDetailedSummary,
}
pub struct Form3PSummary {
    pub line6_cash_on_hand_beginning_period: f64,
    pub line7_total_receipts: f64,
    /// (6 + 7)
    pub line8_subtotal: f64,
    pub line9_total_disbursements: f64,
    pub line10_cash_on_hand_end_period: f64,
    pub line11_debts_owed_to_committee: f64,
    pub line12_debts_owed_by_committee: f64,
    pub line13_expenditures_subject_to_limits: f64,
    pub line14_net_contributions_other_than_loans: f64,
    pub line15_net_operating_expenditures: f64,
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

use these inside data hashmaps

      "col_a_cash_on_hand_beginning_period",
      "col_a_total_receipts",
      "col_a_subtotal",
      "col_a_total_disbursements",
      "col_a_cash_on_hand_close_of_period",
      "col_a_debts_to",
      "col_a_debts_by",
      "col_a_expenditures_subject_to_limits",
      "col_a_net_contributions",
      "col_a_net_operating_expenditures",
      "col_a_federal_funds",
      "col_a_individuals_itemized",
      "col_a_individuals_unitemized",
      "col_a_individual_contribution_total",
      "col_a_political_party_committees_receipts",
      "col_a_other_political_committees_pacs",
      "col_a_the_candidate",
      "col_a_total_contributions",
      "col_a_transfers_from_aff_other_party_cmttees",
      "col_a_received_from_or_guaranteed_by_cand",
      "col_a_other_loans",
      "col_a_total_loans",
      "col_a_operating",
      "col_a_fundraising",
      "col_a_legal_and_accounting",
      "col_a_total_offsets_to_expenditures",
      "col_a_other_receipts",
      "col_a_total_receipts_TODO_DUP",
      "col_a_operating_expenditures",
      "col_a_transfers_to_other_authorized_committees",
      "col_a_fundraising_disbursements",
      "col_a_exempt_legal_accounting_disbursement",
      "col_a_made_or_guaranteed_by_candidate",
      "col_a_other_repayments",
      "col_a_total_loan_repayments_made",
      "col_a_individuals",
      "col_a_political_party_committees_refunds",
      "col_a_other_political_committees",
      "col_a_total_contributions_refunds",
      "col_a_other_disbursements",
      "col_a_total_disbursements_TODO_DUP",
      "col_a_items_on_hand_to_be_liquidated",

      "col_a_totals",

      "col_b_federal_funds",
      "col_b_individuals_itemized",
      "col_b_individuals_unitemized",
      "col_b_individual_contribution_total",
      "col_b_political_party_committees_receipts",
      "col_b_other_political_committees_pacs",
      "col_b_the_candidate",
      "col_b_total_contributions_other_than_loans",
      "col_b_transfers_from_aff_other_party_cmttees",
      "col_b_received_from_or_guaranteed_by_cand",
      "col_b_other_loans",
      "col_b_total_loans",
      "col_b_operating",
      "col_b_fundraising",
      "col_b_legal_and_accounting",
      "col_b_total_offsets_to_operating_expenditures",
      "col_b_other_receipts",
      "col_b_total_receipts",
      "col_b_operating_expenditures",
      "col_b_transfers_to_other_authorized_committees",
      "col_b_fundraising_disbursements",
      "col_b_exempt_legal_accounting_disbursement",
      "col_b_made_or_guaranteed_by_the_candidate",
      "col_b_other_repayments",
      "col_b_total_loan_repayments_made",
      "col_b_individuals",
      "col_b_political_party_committees_refunds",
      "col_b_other_political_committees",
      "col_b_total_contributions_refunds",
      "col_b_other_disbursements",
      "col_b_total_disbursements",

      "col_b_totals"
*/
pub struct Form3PDetailedSummaryReceipts {
    pub line16_federal_funds: DetailedSummaryRow,
    pub line17a_i_contributions_from_individuals_itemized: DetailedSummaryRow,
    pub line17a_ii_contributions_from_individuals_unitemized: DetailedSummaryRow,
    pub line17a_iii_contributions_from_individuals_total: DetailedSummaryRow,
    pub line17b_political_party_committees: DetailedSummaryRow,
    pub line17c_other_political_committees: DetailedSummaryRow,
    pub line17d_the_candidate: DetailedSummaryRow,
    pub line17e_total_contributions: DetailedSummaryRow,
    pub line18_transfers_from_other_authorized_committee: DetailedSummaryRow,
    pub line19a_loans_received_from_or_guaranteed_by_candidate: DetailedSummaryRow,
    pub line19b_other_loans: DetailedSummaryRow,
    pub line19c_total_loans: DetailedSummaryRow,
    pub line20a_offsets_to_expenditures_operating: DetailedSummaryRow,
    pub line20b_offsets_to_expenditures_fundraising: DetailedSummaryRow,
    pub line20c_offsets_to_expenditures_legal_and_accounting: DetailedSummaryRow,
    pub line20d_offsets_to_expenditures_total: DetailedSummaryRow,
    pub line21_other_receipts: DetailedSummaryRow,
    pub line22_total_receipts: DetailedSummaryRow,
}

impl Form3PDetailedSummaryReceipts {
    pub fn from_data(data: &IndexMap<String, String>) -> Self {
        Self {
            line16_federal_funds: DetailedSummaryRow::from_data(
                data,
                "col_a_federal_funds",
                "col_b_federal_funds",
            ),
            line17a_i_contributions_from_individuals_itemized: DetailedSummaryRow::from_data(
                data,
                "col_a_individuals_itemized",
                "col_b_individuals_itemized",
            ),
            line17a_ii_contributions_from_individuals_unitemized: DetailedSummaryRow::from_data(
                data,
                "col_a_individuals_unitemized",
                "col_b_individuals_unitemized",
            ),
            line17a_iii_contributions_from_individuals_total: DetailedSummaryRow::from_data(
                data,
                "col_a_individual_contribution_total",
                "col_b_individual_contribution_total",
            ),
            line17b_political_party_committees: DetailedSummaryRow::from_data(
                data,
                "col_a_political_party_committees_receipts",
                "col_b_political_party_committees_receipts",
            ),
            line17c_other_political_committees: DetailedSummaryRow::from_data(
                data,
                "col_a_other_political_committees_pacs",
                "col_b_other_political_committees_pacs",
            ),
            line17d_the_candidate: DetailedSummaryRow::from_data(
                data,
                "col_a_the_candidate",
                "col_b_the_candidate",
            ),
            line17e_total_contributions: DetailedSummaryRow::from_data(
                data,
                "col_a_total_contributions",
                "col_b_total_contributions_other_than_loans",
            ),
            line18_transfers_from_other_authorized_committee: DetailedSummaryRow::from_data(
                data,
                "col_a_transfers_from_aff_other_party_cmttees",
                "col_b_transfers_from_aff_other_party_cmttees",
            ),
            line19a_loans_received_from_or_guaranteed_by_candidate: DetailedSummaryRow::from_data(
                data,
                "col_a_received_from_or_guaranteed_by_cand",
                "col_b_received_from_or_guaranteed_by_cand",
            ),
            line19b_other_loans: DetailedSummaryRow::from_data(
                data,
                "col_a_other_loans",
                "col_b_other_loans",
            ),
            line19c_total_loans: DetailedSummaryRow::from_data(
                data,
                "col_a_total_loans",
                "col_b_total_loans",
            ),
            line20a_offsets_to_expenditures_operating: DetailedSummaryRow::from_data(
                data,
                "col_a_operating",
                "col_b_operating",
            ),
            line20b_offsets_to_expenditures_fundraising: DetailedSummaryRow::from_data(
                data,
                "col_a_fundraising",
                "col_b_fundraising",
            ),
            line20c_offsets_to_expenditures_legal_and_accounting: DetailedSummaryRow::from_data(
                data,
                "col_a_legal_and_accounting",
                "col_b_legal_and_accounting",
            ),
            line20d_offsets_to_expenditures_total: DetailedSummaryRow::from_data(
                data,
                "col_a_total_offsets_to_expenditures",
                "col_b_total_offsets_to_operating_expenditures",
            ),
            line21_other_receipts: DetailedSummaryRow::from_data(
                data,
                "col_a_other_receipts",
                "col_b_other_receipts",
            ),
            line22_total_receipts: DetailedSummaryRow::from_data(
                data,
                "col_a_total_receipts_TODO_DUP",
                "col_b_total_receipts",
            ),
        }
    }
}

pub struct Form3PDetailedSummaryDisbursements {
    pub line_23_operating_expenditures: DetailedSummaryRow,
    pub line_24_transfers_to_other_authorized_committee: DetailedSummaryRow,
    pub line_25_fundraising: DetailedSummaryRow,
    pub line_26_exempt_legal_and_accounting: DetailedSummaryRow,
    pub line_27a_loan_repayments_from_candidate: DetailedSummaryRow,
    pub line_27b_loan_repayments_other: DetailedSummaryRow,
    pub line_27c_loan_repayments_total: DetailedSummaryRow,
    pub line_28a_refund_contributions_individuals: DetailedSummaryRow,
    pub line_28b_refund_contributions_political_party_committees: DetailedSummaryRow,
    pub line_28c_refund_contributions_other_political_committees: DetailedSummaryRow,
    pub line_28d_refund_contributions_total: DetailedSummaryRow,
    pub line_29_other_disbursements: DetailedSummaryRow,
    pub line_30_total_disbursements: DetailedSummaryRow,
}
impl Form3PDetailedSummaryDisbursements {
    pub fn from_data(_data: &IndexMap<String, String>) -> Self {
        todo!()
    }
}

pub struct Form3PDetailedSummary {
    pub receipts: Form3PDetailedSummaryReceipts,
    //pub disbursements: Form3PDetailedSummaryDisbursements,
    pub items_on_hand_to_be_liquidated: f64,
}

impl Form3PDetailedSummary {
    pub fn from_data(data: &IndexMap<String, String>) -> Self {
        Self {
            receipts: Form3PDetailedSummaryReceipts::from_data(data),
            //disbursements: Form3PDetailedSummaryDisbursements::from_data(data),
            items_on_hand_to_be_liquidated: data["col_a_items_on_hand_to_be_liquidated"]
                .parse()
                .unwrap_or(0.0),
        }
    }
}

impl Form3P {
    pub fn from_data(data: &IndexMap<String, String>) -> Option<Self> {
        let signed = data
            .get("date_signed")
            .and_then(|s| Date::strptime("%Y%m%d", s).ok())?;

        let summary = Form3PSummary {
            line6_cash_on_hand_beginning_period: data
                .get("col_a_cash_on_hand_beginning_period")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line7_total_receipts: data
                .get("col_a_total_receipts")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line8_subtotal: data
                .get("col_a_subtotal")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line9_total_disbursements: data
                .get("col_a_total_disbursements")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line10_cash_on_hand_end_period: data
                .get("col_a_cash_on_hand_close_of_period")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line11_debts_owed_to_committee: data
                .get("col_a_debts_to")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line12_debts_owed_by_committee: data
                .get("col_a_debts_by")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line13_expenditures_subject_to_limits: data
                .get("col_a_expenditures_subject_to_limits")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line14_net_contributions_other_than_loans: data
                .get("col_a_net_contributions")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
            line15_net_operating_expenditures: data
                .get("col_a_net_operating_expenditures")
                .and_then(|s| s.parse::<f64>().ok())
                .unwrap_or(0.0),
        };

        Some(Self {
            treasurer: Treasurer::from_data(data),
            signed,
            summary,
            detailed_summary: Form3PDetailedSummary::from_data(data),
        })
    }
}
