use crate::{cli::ExportArgs, sourcer::FilingSourcer};
use fec_parser::{
    mappings::{column_names_for_field, DATE_COLUMNS, FLOAT_COLUMNS},
    schedules::{form_type_schedule_type, ScheduleType},
    Filing, FilingRow,
};
use rust_xlsxwriter::{worksheet::Worksheet, ExcelDateTime, Format, Workbook};
use std::{collections::HashMap, io::Read, path::PathBuf};

struct ScheduleSheetState {
    worksheet: Worksheet,
    row_idx: u32,
    column_types: Vec<FieldFormat>,
}

#[derive(Clone, Copy)]
enum FieldFormat {
    Text,
    Float,
    Date,
}

fn write_schedule_row(
    sheets: &mut HashMap<ScheduleType, ScheduleSheetState>,
    schedule: ScheduleType,
    filing_fec_version: &str,
    row: &FilingRow,
) -> anyhow::Result<()> {
    let state = sheets.entry(schedule).or_insert_with(|| {
        let mut new_ws = Worksheet::new();
        new_ws
            .set_name(match schedule {
                ScheduleType::ScheduleA => "Schedule A",
                ScheduleType::ScheduleB => "Schedule B",
                ScheduleType::ScheduleC => "Schedule C",
                ScheduleType::ScheduleC1 => "Schedule C1",
                ScheduleType::ScheduleC2 => "Schedule C2",
                ScheduleType::ScheduleD => "Schedule D",
                ScheduleType::ScheduleE => "Schedule E",
                ScheduleType::ScheduleF => "Schedule F",
            })
            .unwrap();

        let column_names = column_names_for_field(&row.row_type, filing_fec_version).unwrap();
        let column_types: Vec<FieldFormat> = column_names
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

        for (idx, column_name) in column_names.iter().enumerate() {
            // never more than 65535 columns, so u16 is fine
            new_ws.write_string(0, idx as u16, column_name).unwrap();
        }

        ScheduleSheetState {
            row_idx: 1,
            worksheet: new_ws,
            column_types,
        }
    });

    for (idx, v) in row.record.iter().enumerate() {
        state.worksheet.write_string(state.row_idx, idx as u16, v)?;
        match state.column_types.get(idx) {
            Some(FieldFormat::Float) => {
                if let Ok(num) = v.parse::<f64>() {
                    state.worksheet.write_number_with_format(
                        state.row_idx,
                        idx as u16,
                        num,
                        &Format::new().set_num_format("$,0.00"),
                    )?;
                }
            }
            Some(FieldFormat::Date) => match jiff::civil::Date::strptime("%Y%m%d", v) {
                Ok(date) => {
                    let d = ExcelDateTime::from_ymd(
                        date.year().try_into().unwrap(),
                        date.month().try_into().unwrap(),
                        date.day().try_into().unwrap(),
                    )
                    .unwrap();
                    state.worksheet.write_datetime_with_format(
                        state.row_idx,
                        idx as u16,
                        d,
                        &Format::new().set_num_format("yyyy-mm-dd"),
                    )?;
                }
                Err(_) => {
                    state.worksheet.write_string(state.row_idx, idx as u16, v)?;
                }
            },
            None | Some(FieldFormat::Text) => {
                state.worksheet.write_string(state.row_idx, idx as u16, v)?;
            }
        }
    }
    state.row_idx += 1;

    /*
    let state:&mut ScheduleSheetState = match sheets.get_mut(&schedule) {

      // schedule worksheet already exists, so just append row
      //Some(ScheduleSheetState{row_idx, worksheet}) => {}
      Some(state) => state,
      // schedule worksheet does not exist, so create it
      None => {
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

        let cols = column_names_for_field(&row.row_type, filing_fec_version).unwrap();
        for (idx, col) in cols.iter().enumerate() {
          let col_idx = (idx).try_into().unwrap();
          new_ws.write_string(0, col_idx, col)?;
        }
        for (idx, v) in row.record.iter().enumerate() {
          new_ws.write_string(1, (idx).try_into().unwrap(), v)?;
        }
        sheets.entry(schedule)
        sheets.insert(schedule, ScheduleSheetState {row_idx: 2, worksheet: new_ws});
      }
    };

    for (idx, v) in row.record.iter().enumerate() {
      worksheet.write_string(*row_idx, (idx).try_into().unwrap(), v)?;
    }
    (*row_idx) += 1;
     */
    Ok(())
}

fn write_form_type_row(
    sheets: &mut HashMap<String, (usize, Worksheet)>,
    filing_fec_version: &str,
    row: &FilingRow,
) -> anyhow::Result<()> {
    match sheets.get_mut(&row.row_type) {
        // schedule worksheet already exists, so just append row
        Some((idx, worksheet)) => {
            let row_idx = (*idx).try_into().unwrap();

            let mut col_idx = 0;
            worksheet.write_string(row_idx, col_idx, "TODO")?;
            col_idx += 1;

            for v in &row.record {
                worksheet.write_string(row_idx, col_idx, v)?;
                //worksheet.write_number_with_format(row, col, number, format)
                col_idx += 1
            }
            (*idx) += 1;
        }
        None => {
            let mut new_ws = Worksheet::new();
            new_ws.set_name(row.row_type.clone())?;

            // header row
            let cols = column_names_for_field(&row.row_type, filing_fec_version).unwrap();
            let mut col_idx: u16 = 0;
            new_ws.write_string(0, col_idx, "filing_id")?;
            col_idx += 1;
            for col in cols {
                new_ws.write_string(0, col_idx, col)?;
                col_idx += 1;
            }

            let mut col_idx = 0;
            new_ws.write_string(1, col_idx, "TODO")?;
            col_idx += 1;
            for v in &row.record {
                new_ws.write_string(1, col_idx, v)?;
                col_idx += 1;
            }

            sheets.insert(row.row_type.clone(), (2, new_ws));
        }
    };
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
        ws.write_string((2 + idx).try_into().unwrap(), 0, k)?;
        ws.write_string((2 + idx).try_into().unwrap(), 1, v)?;
    }
    Ok(())
}

pub fn cmd_export_excel(
    mut sourcer: FilingSourcer,
    path: PathBuf,
    args: ExportArgs,
) -> anyhow::Result<()> {
    let mb = indicatif::MultiProgress::new();
    let mut iter = sourcer
        .resolve_iterator_from_flags(args.filings, args.api, Some(&mb))?
        .1;
    let mut filing = iter.next().unwrap().unwrap();
    if iter.next().is_some() {
        return Err(anyhow::anyhow!(
            "Only one filing supported for Excel export"
        ));
    }

    let mut workbook = Workbook::new();
    add_summary_worksheet(&mut workbook, &filing)?;

    let mut schedule_sheets: HashMap<ScheduleType, ScheduleSheetState> = HashMap::new();
    let mut rowtype_sheets: HashMap<String, (usize, Worksheet)> = HashMap::new();

    while let Some(Ok(row)) = filing.next_row() {
        match form_type_schedule_type(&row.row_type) {
            Some(schedule) => {
                write_schedule_row(
                    &mut schedule_sheets,
                    schedule,
                    &filing.header.fec_version,
                    &row,
                )?;
            }
            None => {
                write_form_type_row(&mut rowtype_sheets, &filing.header.fec_version, &row)?;
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
