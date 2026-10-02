use crate::{
    cli::ExportArgs,
    sourcer::FilingSourcer,
    utils::rows::{legacy_name_delimiter, normalize_fec_date, remap_row, warn, UnmappedRows},
};
use fec_parser::{
    mappings::{column_names_for_field, DATE_COLUMNS, FLOAT_COLUMNS},
    schedules::{form_type_schedule_type, ScheduleType},
    Filing, FilingRow,
};
use rust_xlsxwriter::{worksheet::Worksheet, ExcelDateTime, Format, Workbook};
use std::{
    collections::{hash_map::Entry, HashMap},
    io::Read,
    path::PathBuf,
};

struct ScheduleSheetState {
    worksheet: Worksheet,
    row_idx: u32,
    columns: Vec<String>,
    column_types: Vec<FieldFormat>,
}

#[derive(Clone, Copy)]
enum FieldFormat {
    Text,
    Float,
    Date,
}

/// A worksheet name Excel accepts: no `[]:*?/\\`, at most 31 characters.
fn sheet_name(name: &str) -> String {
    name.chars()
        .map(|c| if "[]:*?/\\".contains(c) { '-' } else { c })
        .take(31)
        .collect()
}

fn write_date(ws: &mut Worksheet, row: u32, col: u16, v: &str) -> anyhow::Result<()> {
    let date = normalize_fec_date(v)
        .and_then(|d| jiff::civil::Date::strptime("%Y-%m-%d", d).ok())
        .and_then(|date| {
            ExcelDateTime::from_ymd(
                date.year().try_into().ok()?,
                date.month().try_into().ok()?,
                date.day().try_into().ok()?,
            )
            .ok()
        });
    match date {
        Some(d) => {
            ws.write_datetime_with_format(row, col, d, &Format::new().set_num_format("yyyy-mm-dd"))?;
        }
        None => {
            ws.write_string(row, col, v)?;
        }
    }
    Ok(())
}

/// Column index as Excel's u16, or an error past its limit.
fn col(idx: usize) -> anyhow::Result<u16> {
    u16::try_from(idx).map_err(|_| anyhow::anyhow!("Too many columns for Excel: {idx}"))
}

fn write_schedule_row(
    sheets: &mut HashMap<ScheduleType, ScheduleSheetState>,
    schedule: ScheduleType,
    row_columns: &[String],
    row: &FilingRow,
    name_delimiter: Option<&str>,
) -> anyhow::Result<()> {
    let state = match sheets.entry(schedule) {
        Entry::Occupied(e) => e.into_mut(),
        Entry::Vacant(e) => {
            let mut new_ws = Worksheet::new();
            new_ws.set_name(match schedule {
                ScheduleType::ScheduleA => "Schedule A",
                ScheduleType::ScheduleB => "Schedule B",
                ScheduleType::ScheduleC => "Schedule C",
                ScheduleType::ScheduleC1 => "Schedule C1",
                ScheduleType::ScheduleC2 => "Schedule C2",
                ScheduleType::ScheduleD => "Schedule D",
                ScheduleType::ScheduleE => "Schedule E",
                ScheduleType::ScheduleF => "Schedule F",
            })?;

            let columns = row_columns.to_vec();
            let column_types: Vec<FieldFormat> = columns
                .iter()
                .map(|c| {
                    if DATE_COLUMNS.contains(c) {
                        FieldFormat::Date
                    } else if FLOAT_COLUMNS.contains(c) {
                        FieldFormat::Float
                    } else {
                        FieldFormat::Text
                    }
                })
                .collect();

            for (idx, column_name) in columns.iter().enumerate() {
                new_ws.write_string(0, col(idx)?, column_name)?;
            }

            e.insert(ScheduleSheetState {
                row_idx: 1,
                worksheet: new_ws,
                columns,
                column_types,
            })
        }
    };

    let fields = remap_row(&state.columns, row_columns, &row.record, name_delimiter);
    for (idx, v) in fields.into_iter().enumerate() {
        let c = col(idx)?;
        match state.column_types.get(idx) {
            Some(FieldFormat::Float) => match v.parse::<f64>() {
                Ok(num) => {
                    state.worksheet.write_number_with_format(
                        state.row_idx,
                        c,
                        num,
                        &Format::new().set_num_format("$,0.00"),
                    )?;
                }
                Err(_) => {
                    state.worksheet.write_string(state.row_idx, c, &*v)?;
                }
            },
            Some(FieldFormat::Date) => write_date(&mut state.worksheet, state.row_idx, c, &v)?,
            None | Some(FieldFormat::Text) => {
                state.worksheet.write_string(state.row_idx, c, &*v)?;
            }
        }
    }
    state.row_idx += 1;
    Ok(())
}

fn write_form_type_row(
    sheets: &mut HashMap<String, (u32, Worksheet)>,
    filing_id: &str,
    row_columns: &[String],
    row: &FilingRow,
) -> anyhow::Result<()> {
    let (row_idx, worksheet) = match sheets.entry(row.row_type.clone()) {
        Entry::Occupied(e) => e.into_mut(),
        Entry::Vacant(e) => {
            let mut new_ws = Worksheet::new();
            new_ws.set_name(sheet_name(&row.row_type))?;
            new_ws.write_string(0, 0, "filing_id")?;
            for (idx, name) in row_columns.iter().enumerate() {
                new_ws.write_string(0, col(idx + 1)?, name)?;
            }
            e.insert((1, new_ws))
        }
    };
    worksheet.write_string(*row_idx, 0, filing_id)?;
    for (idx, v) in row.record.iter().enumerate() {
        worksheet.write_string(*row_idx, col(idx + 1)?, v)?;
    }
    *row_idx += 1;
    Ok(())
}

fn add_summary_worksheet(
    workbook: &mut Workbook,
    filing: &Filing<Box<dyn Read>>,
) -> anyhow::Result<()> {
    let ws = workbook.add_worksheet();
    ws.set_name("Summary")?;
    ws.write_string(0, 0, &filing.filing_id)?;

    for (idx, (k, v)) in filing.cover.cover_record_kv.iter().enumerate() {
        let row = u32::try_from(2 + idx)?;
        ws.write_string(row, 0, k)?;
        ws.write_string(row, 1, v)?;
    }
    Ok(())
}

pub fn cmd_export_excel(
    mut sourcer: FilingSourcer,
    path: PathBuf,
    args: ExportArgs,
) -> anyhow::Result<()> {
    let mb = indicatif::MultiProgress::new();
    let (_trace, _input_mappings, mut iter) =
        sourcer.resolve_iterator_from_flags(args.filings, args.api, Some(&mb))?;
    let mut filing = iter
        .next()
        .ok_or_else(|| anyhow::anyhow!("No filing to export"))??;
    if iter.next().is_some() {
        return Err(anyhow::anyhow!(
            "Only one filing supported for Excel export"
        ));
    }

    let mut workbook = Workbook::new();
    add_summary_worksheet(&mut workbook, &filing)?;

    let mut schedule_sheets: HashMap<ScheduleType, ScheduleSheetState> = HashMap::new();
    let mut rowtype_sheets: HashMap<String, (u32, Worksheet)> = HashMap::new();
    let mut unmapped = UnmappedRows::new(Some(&mb));
    let fec_version = filing.header.fec_version.clone();

    while let Some(row) = filing.next_row() {
        let row = match row {
            Ok(row) => row,
            Err(e) => {
                warn(
                    Some(&mb),
                    format!(
                        "warning: FEC-{}: skipping unreadable row: {e}",
                        filing.filing_id
                    ),
                );
                continue;
            }
        };
        let Ok(row_columns) = column_names_for_field(&row.row_type, &fec_version) else {
            unmapped.warn(&filing.filing_id, &row.row_type, &fec_version, "skipping them");
            continue;
        };
        match form_type_schedule_type(&row.row_type) {
            Some(schedule) => {
                write_schedule_row(
                    &mut schedule_sheets,
                    schedule,
                    row_columns,
                    &row,
                    legacy_name_delimiter(&filing.header),
                )?;
            }
            None => {
                write_form_type_row(&mut rowtype_sheets, &filing.filing_id, row_columns, &row)?;
            }
        };
    }

    let mut schedule_sheets: Vec<(ScheduleType, ScheduleSheetState)> =
        schedule_sheets.into_iter().collect();
    schedule_sheets.sort_by_key(|(_, ws)| ws.worksheet.name());

    for (
        _,
        ScheduleSheetState {
            worksheet: mut ws, ..
        },
    ) in schedule_sheets
    {
        ws.autofit_to_max_width(300);
        workbook.push_worksheet(ws);
    }

    for (_, (_, ws)) in rowtype_sheets.into_iter() {
        workbook.push_worksheet(ws);
    }

    workbook.save(path)?;
    Ok(())
}
