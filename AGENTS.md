# Agent Guidelines

## TUI Code Patterns

When building TUI interfaces with `ratatui`, follow these conventions:

### Layout Definition

Define layouts declaratively using `Layout::default()` with explicit constraints. Add comments explaining what each constraint represents:

```rust
let layout = Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3), // Input and cycle selection
        Constraint::Length(1), // Tabs
        Constraint::Min(1),    // Results table
        Constraint::Length(2), // Help text
    ]);
```

### Area Destructuring

Use `.layout(&layout)` on an area to destructure into named sections. This makes the code self-documenting:

```rust
let [top_bar, tabs, results_table, help_text] = f.area().layout(&layout);
```

For nested layouts, apply the same pattern to sub-areas:

```rust
let top_bar_layout = Layout::default()
    .direction(Direction::Horizontal)
    .constraints([Constraint::Min(10), Constraint::Length(15)]);

let [search_bar, cycle_selection] = top_bar.layout(&top_bar_layout);
```

### Render Helper Functions

Create dedicated `render_X()` functions for each UI component. These functions should follow this signature pattern:

```rust
fn render_X(f: &mut Frame, app: &mut App, area: Rect) {
    // Render logic here
    f.render_widget(widget, area);
}
```

- `f: &mut Frame` - the ratatui frame for rendering
- `app: &mut App` - application state (mutable for stateful widgets)
- `area: Rect` - the area to render into

### Composing the View

The main render function should:
1. Define the layout
2. Destructure into named areas
3. Call render helpers in logical order

```rust
fn render_search_view(f: &mut Frame, app: &mut App) {
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([...]);

    let [top_bar, tabs, results_table, help_text] = f.area().layout(&layout);

    render_search_bar(f, app, search_bar);
    render_tabs(f, app, tabs);
    render_results_table(f, app, results_table);
    render_help_text(f, app, help_text);
}
```

### Conditional Rendering

Use `match` for rendering different content based on app state:

```rust
match app.active_tab {
    ResultsTab::Candidates => render_candidate_results_table(f, app, results_table),
    ResultsTab::Committees => render_committee_results_table(f, app, results_table),
}
```

## TUI Testing with Insta Snapshots

Use `insta` and `ratatui::backend::TestBackend` for snapshot testing TUI components.

### Test Module Structure

Add tests at the end of the TUI module file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use insta::assert_snapshot;
    use ratatui::{backend::TestBackend, Terminal};

    #[test]
    fn test_ui_render() {
        let mut app = create_test_app();
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal.draw(|f| ui(f, &mut app)).unwrap();
        assert_snapshot!(terminal.backend());
    }
}
```

### Test App Builder Pattern

Create a builder for constructing test App instances with controlled state:

```rust
struct TestAppBuilder {
    items: Vec<Item>,
    error: Option<String>,
    // ... other configurable fields
}

impl Default for TestAppBuilder {
    fn default() -> Self {
        Self {
            items: Vec::new(),
            error: None,
        }
    }
}

impl TestAppBuilder {
    fn items(mut self, items: Vec<Item>) -> Self {
        self.items = items;
        self
    }

    fn error(mut self, error: &str) -> Self {
        self.error = Some(error.to_string());
        self
    }

    fn build(self) -> App {
        // Construct App with test values
    }
}
```

### Test Data Helpers

Create helper functions for generating realistic test data:

```rust
fn create_test_items() -> Vec<Item> {
    vec![
        Item {
            title: "Test Item 1".to_string(),
            // ... realistic field values
        },
        // ... more items
    ]
}
```

### What to Test

Write snapshot tests for:

1. **Full UI states** - Empty, with data, with errors
2. **Individual render functions** - Test each `render_X()` helper in isolation
3. **UI variations** - With/without filters, popups open/closed
4. **Terminal sizes** - Wide (120x30), standard (80x24), narrow (60x20)

### Example Test Cases

```rust
#[test]
fn test_ui_empty_state() {
    let mut app = TestAppBuilder::default().build();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();
    assert_snapshot!(terminal.backend());
}

#[test]
fn test_ui_with_error() {
    let mut app = TestAppBuilder::default()
        .error("Connection timeout")
        .build();
    let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
    terminal.draw(|f| ui(f, &mut app)).unwrap();
    assert_snapshot!(terminal.backend());
}

#[test]
fn test_render_header() {
    let app = TestAppBuilder::default().build();
    let mut terminal = Terminal::new(TestBackend::new(80, 5)).unwrap();
    terminal
        .draw(|f| render_header(f, &app, f.area()))
        .unwrap();
    assert_snapshot!(terminal.backend());
}
```

### Running Tests

```bash
# Run tests (will fail on first run for new snapshots)
cargo test -p fec-cli module::tests

# Accept new/changed snapshots
cargo insta test -p fec-cli --accept

# Review snapshots interactively
cargo insta review
```

Snapshots are stored in `snapshots/` directory alongside the test file.

## Reusable TUI Components with Self-Contained Keypress Handling

When a TUI component (like a detail view) is used in multiple places with different parent apps, centralize keypress handling in the component itself using an action enum pattern.

### Problem

Detail views like `CommitteeDetailState` may be embedded in:
- Standalone TUI apps (e.g., `fec info C00401224`)
- Larger apps with multiple views (e.g., `fec search` → select → detail view)

Duplicating keypress handling in each parent leads to inconsistency and maintenance burden.

### Solution: Action Enum + `handle_key_event()`

1. **Define an action enum** in the component module:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommitteeDetailAction {
    None,         // Key handled internally, no parent action needed
    Exit,         // User wants to exit/go back
    OpenBrowser,  // User pressed 'o' to open in browser
}
```

2. **Add `handle_key_event()` to the state struct**:

```rust
impl CommitteeDetailState {
    pub fn handle_key_event(
        &mut self,
        key: KeyEvent,
        data: &CommitteeDetail,
    ) -> CommitteeDetailAction {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') => {
                if self.show_popup {
                    self.show_popup = false;
                    CommitteeDetailAction::None
                } else {
                    CommitteeDetailAction::Exit
                }
            }
            KeyCode::Char('o') => CommitteeDetailAction::OpenBrowser,
            KeyCode::Char('y') => {
                self.show_popup = true;
                CommitteeDetailAction::None
            }
            // ... other keys
            _ => CommitteeDetailAction::None,
        }
    }
}
```

3. **Parent apps handle only the actions they care about**:

```rust
// Standalone TUI (info command)
loop {
    terminal.draw(|f| render_committee_detail(f, area, &detail, &state))?;

    if let Event::Key(key) = event::read()? {
        match state.handle_key_event(key, &detail) {
            CommitteeDetailAction::Exit => break,
            CommitteeDetailAction::OpenBrowser => { detail.open_in_browser(); }
            CommitteeDetailAction::None => {}
        }
    }
}

// Embedded in larger app (search command)
match app.view_state {
    ViewState::CommitteeDetail => {
        if let Some(ref committee) = app.committee_detail {
            match app.committee_detail_state.handle_key_event(key, committee) {
                CommitteeDetailAction::Exit => app.go_back_to_search(),
                CommitteeDetailAction::OpenBrowser => { committee.open_in_browser(); }
                CommitteeDetailAction::None => {}
            }
        }
        continue; // Don't fall through to other key handling
    }
    ViewState::Search => { /* search key handling */ }
}
```

### Benefits

- **Single source of truth**: All component-specific keys defined once
- **Consistency**: Same behavior everywhere the component is used
- **Extensibility**: Adding new keys only requires updating the component
- **Clean separation**: Parent decides what `Exit` means in its context
