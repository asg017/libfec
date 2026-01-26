use crate::cli::RssArgs;
use crate::commands::export::sqlite;
use crate::rss::{self, format_duration_ago, ActiveFilters, Item};
use crate::sourcer::FilingSourcer;
use anyhow::Result;
use jiff::{Timestamp, Zoned};
use ratatui::widgets::TableState;
use rusqlite::Connection;
use std::collections::HashSet;
use std::time::{Duration, Instant};

/// Copy menu options
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CopyOption {
    FilingId,
    CommitteeId,
    RssGuid,
}

impl CopyOption {
    pub fn all() -> &'static [CopyOption] {
        &[
            CopyOption::FilingId,
            CopyOption::CommitteeId,
            CopyOption::RssGuid,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            CopyOption::FilingId => "Filing ID",
            CopyOption::CommitteeId => "Committee ID",
            CopyOption::RssGuid => "RSS GUID",
        }
    }
}

/// Application state for the RSS TUI
pub struct App {
    /// The fetched feed items
    pub items: Vec<Item>,
    /// Feed title
    pub feed_title: String,
    /// Last-Modified timestamp from server
    pub last_modified: Option<Timestamp>,
    /// When we last fetched
    pub last_fetch: Instant,
    /// When to fetch next
    pub next_fetch: Instant,
    /// Refresh interval
    pub interval: Duration,
    /// Max items to display
    pub limit: usize,
    /// Table selection state
    pub table_state: TableState,
    /// Whether to exit
    pub should_exit: bool,
    /// Error message to display
    pub error: Option<String>,
    /// Command-line args for fetching
    pub args: RssArgs,
    /// Active filters being used
    pub active_filters: ActiveFilters,
    /// URL used to fetch data
    pub feed_url: String,
    /// Filing sourcer for loading filing details
    pub sourcer: FilingSourcer,
    /// Last key pressed (for `gg` detection)
    pub last_key: Option<crossterm::event::KeyCode>,
    /// Copy menu state
    pub copy_menu_open: bool,
    /// Copy menu selection index
    pub copy_menu_selection: usize,
    /// Status message to show briefly
    pub status_message: Option<String>,
    /// SQLite database connection for export (if -x flag is set)
    pub export_db: Option<Connection>,
    /// Set of filing IDs already exported
    pub exported_ids: HashSet<String>,
    /// Whether to export cover only
    pub cover_only: bool,
    /// Count of filings exported this session
    pub export_count: usize,
    /// Queue of filing IDs pending export
    pub export_queue: Vec<String>,
    /// Total filings to export in current batch (for progress display)
    pub export_batch_total: usize,
}

impl App {
    pub fn new(
        args: RssArgs,
        sourcer: FilingSourcer,
        export_db: Option<Connection>,
        exported_ids: HashSet<String>,
    ) -> Self {
        let now = Instant::now();
        let interval = args.interval;
        let limit = args.limit;
        let cover_only = args.cover_only;
        let (feed_url, _) = rss::build_feed_url(&args);
        Self {
            items: Vec::new(),
            feed_title: String::new(),
            last_modified: None,
            last_fetch: now,
            next_fetch: now, // Fetch immediately
            interval: Duration::from_secs(interval),
            limit,
            table_state: TableState::default(),
            should_exit: false,
            error: None,
            args,
            active_filters: ActiveFilters::default(),
            feed_url,
            sourcer,
            last_key: None,
            copy_menu_open: false,
            copy_menu_selection: 0,
            status_message: None,
            export_db,
            exported_ids,
            cover_only,
            export_count: 0,
            export_queue: Vec::new(),
            export_batch_total: 0,
        }
    }

    pub fn fetch(&mut self) -> Result<()> {
        match rss::fetch_feed_with_args(&self.args) {
            Ok((result, filters)) => {
                self.feed_title = result.feed.title;
                self.last_modified = result.last_modified;
                self.last_fetch = Instant::now();
                self.next_fetch = self.last_fetch + self.interval;
                self.error = None;
                self.active_filters = filters;

                // Queue new filings for export if export is enabled
                if self.export_db.is_some() {
                    self.export_queue.clear();
                    for item in result.feed.items.iter().take(self.limit) {
                        if let Some(ref filing_id) = item.filing_id {
                            if !self.exported_ids.contains(filing_id) {
                                self.export_queue.push(filing_id.clone());
                            }
                        }
                    }
                    self.export_batch_total = self.export_queue.len();
                }

                self.items = result.feed.items;

                // Select first item if available
                if !self.items.is_empty() && self.table_state.selected().is_none() {
                    self.table_state.select(Some(0));
                }
            }
            Err(e) => {
                self.error = Some(e.to_string());
                // Still update timing so we retry
                self.last_fetch = Instant::now();
                self.next_fetch = self.last_fetch + self.interval;
            }
        }
        Ok(())
    }

    pub fn seconds_until_refresh(&self) -> u64 {
        let now = Instant::now();
        if now >= self.next_fetch {
            0
        } else {
            (self.next_fetch - now).as_secs()
        }
    }

    pub fn data_age_seconds(&self) -> i64 {
        if let Some(last_mod) = self.last_modified {
            let now = Zoned::now();
            now.timestamp().duration_since(last_mod).as_secs()
        } else {
            self.last_fetch.elapsed().as_secs() as i64
        }
    }

    pub fn select_next(&mut self) {
        let item_count = self.items.len();
        if item_count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= item_count - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn select_previous(&mut self) {
        let item_count = self.items.len();
        if item_count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    item_count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn select_first(&mut self) {
        if !self.items.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        if !self.items.is_empty() {
            self.table_state.select(Some(self.items.len() - 1));
        }
    }

    pub fn get_selected_item(&self) -> Option<&Item> {
        self.table_state.selected().and_then(|i| self.items.get(i))
    }

    pub fn copy_selected(&mut self, option: CopyOption) {
        let value = self.get_selected_item().and_then(|item| match option {
            CopyOption::FilingId => item.filing_id.clone(),
            CopyOption::CommitteeId => item.committee_id.clone(),
            CopyOption::RssGuid => Some(item.guid.clone()),
        });

        if let Some(text) = value {
            if let Ok(mut ctx) = arboard::Clipboard::new() {
                if ctx.set_text(&text).is_ok() {
                    self.status_message = Some(format!("Copied: {}", text));
                } else {
                    self.status_message = Some("Failed to copy".to_string());
                }
            } else {
                self.status_message = Some("Clipboard unavailable".to_string());
            }
        } else {
            self.status_message = Some("No value to copy".to_string());
        }
        self.copy_menu_open = false;
    }

    pub fn copy_menu_next(&mut self) {
        let count = CopyOption::all().len();
        self.copy_menu_selection = (self.copy_menu_selection + 1) % count;
    }

    pub fn copy_menu_prev(&mut self) {
        let count = CopyOption::all().len();
        self.copy_menu_selection = if self.copy_menu_selection == 0 {
            count - 1
        } else {
            self.copy_menu_selection - 1
        };
    }

    /// Returns true if there are pending exports
    pub fn has_pending_exports(&self) -> bool {
        !self.export_queue.is_empty()
    }

    /// Returns export progress as (completed, total) for current batch
    pub fn export_progress(&self) -> (usize, usize) {
        let completed = self.export_batch_total - self.export_queue.len();
        (completed, self.export_batch_total)
    }

    /// Process one pending export from the queue
    pub fn process_one_export(&mut self) {
        if let Some(filing_id) = self.export_queue.pop() {
            let (completed, total) = self.export_progress();
            self.status_message = Some(format!(
                "Exporting {}/{}: {}...",
                completed + 1,
                total,
                filing_id
            ));

            if let Some(ref mut db) = self.export_db {
                match self.sourcer.resolve_from_user_argument(&filing_id) {
                    Ok(filing) => match sqlite::export_single_filing(db, filing, self.cover_only) {
                        Ok(_) => {
                            self.exported_ids.insert(filing_id);
                            self.export_count += 1;
                        }
                        Err(e) => {
                            self.error = Some(format!("Export error: {}", e));
                        }
                    },
                    Err(e) => {
                        self.error = Some(format!("Fetch error for {}: {}", filing_id, e));
                    }
                }
            }

            // Show completion message when done
            if self.export_queue.is_empty() && self.export_batch_total > 0 {
                self.status_message =
                    Some(format!("Exported {} filing(s)", self.export_batch_total));
                self.export_batch_total = 0;
            }
        }
    }

    pub fn data_age_display(&self) -> String {
        format_duration_ago(self.data_age_seconds())
    }
}
