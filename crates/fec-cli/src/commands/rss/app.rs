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

/// Search mode for filtering filings by committee name
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchMode {
    /// No search active
    Off,
    /// Actively typing a search query
    Typing,
    /// Search filter locked in, back to normal navigation
    Locked,
}

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
    /// Only show/export items since this timestamp
    pub since_ts: Option<Timestamp>,
    /// Committee name search query
    pub search_query: String,
    /// Current search mode
    pub search_mode: SearchMode,
    /// Indices into `items` that match the current search query
    pub filtered_indices: Vec<usize>,
}

impl App {
    pub fn new(
        args: RssArgs,
        sourcer: FilingSourcer,
        export_db: Option<Connection>,
        exported_ids: HashSet<String>,
        since_ts: Option<Timestamp>,
    ) -> Self {
        let now = Instant::now();
        let interval = args.interval;
        let cover_only = args.cover_only;
        let (feed_url, _) = rss::build_feed_url(&args);
        Self {
            items: Vec::new(),
            feed_title: String::new(),
            last_modified: None,
            last_fetch: now,
            next_fetch: now, // Fetch immediately
            interval: Duration::from_secs(interval),
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
            since_ts,
            search_query: String::new(),
            search_mode: SearchMode::Off,
            filtered_indices: Vec::new(),
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

                // Refresh exported_ids from database before queuing new exports
                // This ensures we don't re-export filings that were just exported
                if let Some(ref db) = self.export_db {
                    if let Ok(ids) = sqlite::get_existing_filing_ids(db) {
                        self.exported_ids = ids;
                    }
                }

                // Filter items by --since if provided
                let filtered_items: Vec<Item> = if let Some(since) = self.since_ts {
                    result
                        .feed
                        .items
                        .into_iter()
                        .filter(|item| {
                            item.pub_date
                                .map(|pub_date| pub_date >= since)
                                .unwrap_or(false)
                        })
                        .collect()
                } else {
                    result.feed.items
                };

                // Queue new filings for export if export is enabled
                // Export ALL filtered items from feed, not just the displayed limit
                if self.export_db.is_some() {
                    self.export_queue.clear();
                    for item in filtered_items.iter() {
                        if let Some(ref filing_id) = item.filing_id {
                            if !self.exported_ids.contains(filing_id) {
                                self.export_queue.push(filing_id.clone());
                            }
                        }
                    }
                    self.export_batch_total = self.export_queue.len();
                }

                self.items = filtered_items;
                self.update_filter();
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
        let count = self.filtered_indices.len();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i >= count - 1 {
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
        let count = self.filtered_indices.len();
        if count == 0 {
            return;
        }
        let i = match self.table_state.selected() {
            Some(i) => {
                if i == 0 {
                    count - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.table_state.select(Some(i));
    }

    pub fn select_first(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.table_state.select(Some(0));
        }
    }

    pub fn select_last(&mut self) {
        if !self.filtered_indices.is_empty() {
            self.table_state.select(Some(self.filtered_indices.len() - 1));
        }
    }

    pub fn get_selected_item(&self) -> Option<&Item> {
        let selected = self.table_state.selected()?;
        let &item_idx = self.filtered_indices.get(selected)?;
        self.items.get(item_idx)
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

    /// Recompute filtered_indices based on the current search query
    pub fn update_filter(&mut self) {
        if self.search_query.is_empty() {
            self.filtered_indices = (0..self.items.len()).collect();
        } else {
            let query = self.search_query.to_lowercase();
            self.filtered_indices = self
                .items
                .iter()
                .enumerate()
                .filter(|(_, item)| {
                    item.extract_committee_name()
                        .to_lowercase()
                        .contains(&query)
                })
                .map(|(i, _)| i)
                .collect();
        }
        // Keep selection in bounds
        if self.filtered_indices.is_empty() {
            self.table_state.select(None);
        } else {
            let current = self.table_state.selected().unwrap_or(0);
            if current >= self.filtered_indices.len() {
                self.table_state.select(Some(0));
            }
        }
    }

    pub fn start_search(&mut self) {
        self.search_mode = SearchMode::Typing;
        self.search_query.clear();
        self.update_filter();
    }

    pub fn search_push_char(&mut self, c: char) {
        self.search_query.push(c);
        self.update_filter();
    }

    pub fn search_pop_char(&mut self) {
        self.search_query.pop();
        self.update_filter();
    }

    pub fn lock_search(&mut self) {
        if self.search_query.is_empty() {
            self.cancel_search();
        } else {
            self.search_mode = SearchMode::Locked;
        }
    }

    pub fn cancel_search(&mut self) {
        self.search_mode = SearchMode::Off;
        self.search_query.clear();
        self.update_filter();
    }
}
