use crate::errors::{TResult, ToGeneric};
use crate::git::cmd::GitCommand;
use crate::gitlab;

pub fn execute_select(ctx: &crate::gitlab::GitContext) -> TResult<String> {
    let mut terminal = ratatui::init();

    let mrs = ctx
        .gitlab_client
        .get_merge_requests(gitlab::client::GetMergeRequestsFilter {
            author: Some(gitlab::client::MergeRequestFilterAuthor::Me),
            opened_only: true,
        })?;

    let result = RatatuiApp::new(&ctx.gitlab_client, mrs).run(&mut terminal);
    ratatui::restore();
    result
}

struct RatatuiApp<'a> {
    gitlab_client: &'a gitlab::client::GitlabProjectClient,
    mrs: Vec<gitlab::client::MergeRequest>,
    visible: Vec<usize>,
    state: ratatui::widgets::TableState,
    status: Option<String>,
    checkout: Option<CheckoutState>,
    input: Option<InputState>,
    author_label: String,
}

impl<'a> RatatuiApp<'a> {
    fn new(
        gitlab_client: &'a gitlab::client::GitlabProjectClient,
        mrs: Vec<gitlab::client::MergeRequest>,
    ) -> Self {
        let has_mrs = !mrs.is_empty();
        let visible: Vec<usize> = (0..mrs.len()).collect();
        let mut state = ratatui::widgets::TableState::default();
        state.select(if has_mrs { Some(0) } else { None });
        Self {
            gitlab_client,
            mrs,
            visible,
            state,
            status: if has_mrs {
                None
            } else {
                Some("No open merge requests found for your user.".to_string())
            },
            checkout: None,
            input: None,
            author_label: "me".to_string(),
        }
    }

    fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> TResult<String> {
        loop {
            self.draw(terminal)?;

            if let Some(result) = self.poll_checkout()? {
                return result;
            }

            if self.checkout.is_some() {
                self.poll_input_during_checkout()?;
                self.advance_spinner();
                continue;
            }

            match self.handle_input()? {
                UiAction::None => {}
                UiAction::Cancel => return Ok("Selection cancelled.".to_string()),
                UiAction::StartCheckout(idx) => {
                    self.start_checkout(idx);
                }
                UiAction::OpenInBrowser(idx) => {
                    self.open_in_browser(idx);
                }
                UiAction::FilterAuthor(query) => {
                    self.load_author_mrs(terminal, &query)?;
                }
            }
        }
    }

    fn draw(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> TResult<()> {
        terminal
            .draw(|frame| {
                let rows = self.visible.iter().map(|&idx| {
                    let mr = &self.mrs[idx];
                    let spinner = self.spinner_for(idx);
                    ratatui::widgets::Row::new(vec![
                        format!("{}{}", spinner, mr.iid),
                        mr.title.clone(),
                        mr.state.as_str().to_string(),
                    ])
                });

                let table = ratatui::widgets::Table::new(
                    rows,
                    vec![
                        ratatui::layout::Constraint::Length(6),
                        ratatui::layout::Constraint::Min(20),
                        ratatui::layout::Constraint::Length(10),
                    ],
                )
                .header(
                    ratatui::widgets::Row::new(vec!["IID", "Title", "State"]).style(
                        ratatui::style::Style::default()
                            .add_modifier(ratatui::style::Modifier::BOLD),
                    ),
                )
                .block(
                    ratatui::widgets::Block::default()
                        .borders(ratatui::widgets::Borders::ALL)
                        .title(format!("Merge Requests ({})", self.author_label)),
                )
                .highlight_symbol(">> ")
                .row_highlight_style(
                    ratatui::style::Style::default()
                        .add_modifier(ratatui::style::Modifier::REVERSED),
                );

                let chunks = ratatui::layout::Layout::default()
                    .direction(ratatui::layout::Direction::Vertical)
                    .constraints([
                        ratatui::layout::Constraint::Min(1),
                        ratatui::layout::Constraint::Length(1),
                    ])
                    .split(frame.area());

                frame.render_stateful_widget(table, chunks[0], &mut self.state);
                let help = ratatui::widgets::Paragraph::new(self.help_text());
                frame.render_widget(help, chunks[1]);
            })
            .to_generic()?;
        Ok(())
    }

    fn handle_input(&mut self) -> TResult<UiAction> {
        match ratatui::crossterm::event::read().to_generic()? {
            ratatui::crossterm::event::Event::Key(key) => {
                if key.kind != ratatui::crossterm::event::KeyEventKind::Press {
                    return Ok(UiAction::None);
                }
                match key.code {
                    _ if self.input.is_some() => self.handle_author_input(key.code),
                    ratatui::crossterm::event::KeyCode::Char('q')
                    | ratatui::crossterm::event::KeyCode::Esc => Ok(UiAction::Cancel),
                    ratatui::crossterm::event::KeyCode::Down
                    | ratatui::crossterm::event::KeyCode::Char('j') => {
                        self.move_selection(1);
                        Ok(UiAction::None)
                    }
                    ratatui::crossterm::event::KeyCode::Up
                    | ratatui::crossterm::event::KeyCode::Char('k') => {
                        self.move_selection(-1);
                        Ok(UiAction::None)
                    }
                    ratatui::crossterm::event::KeyCode::Enter => {
                        let idx = self
                            .state
                            .selected()
                            .and_then(|i| self.visible.get(i).copied());
                        Ok(match idx {
                            Some(idx) => UiAction::StartCheckout(idx),
                            None => UiAction::None,
                        })
                    }
                    ratatui::crossterm::event::KeyCode::Char('v') => {
                        let idx = self.selected_mr_idx();
                        Ok(match idx {
                            Some(idx) => UiAction::OpenInBrowser(idx),
                            None => UiAction::None,
                        })
                    }
                    ratatui::crossterm::event::KeyCode::Char('a') => {
                        self.input = Some(InputState::default());
                        self.status = None;
                        Ok(UiAction::None)
                    }
                    _ => Ok(UiAction::None),
                }
            }
            _ => Ok(UiAction::None),
        }
    }

    fn move_selection(&mut self, delta: isize) {
        let total = self.visible.len();
        if total == 0 {
            self.state.select(None);
            return;
        }
        let idx = self.state.selected().unwrap_or(0);
        let next = if delta >= 0 {
            (idx + 1) % total
        } else if idx == 0 {
            total - 1
        } else {
            idx - 1
        };
        self.state.select(Some(next));
    }

    fn help_text(&self) -> String {
        if let Some(input) = &self.input {
            return format!("Author email or username: {}_", input.value);
        }
        match &self.status {
            Some(status) => status.clone(),
            None => "Up/Down or j/k to move, Enter to checkout, v to view, a to filter author, q/Esc to cancel".to_string(),
        }
    }

    fn set_status(&mut self, status: &str) {
        self.status = Some(status.to_string());
    }

    fn spinner_for(&self, mr_idx: usize) -> &'static str {
        match &self.checkout {
            Some(checkout) if checkout.mr_idx == mr_idx => {
                SPINNER_FRAMES[checkout.tick % SPINNER_FRAMES.len()]
            }
            _ => "",
        }
    }

    fn advance_spinner(&mut self) {
        if let Some(checkout) = &mut self.checkout {
            checkout.tick = checkout.tick.wrapping_add(1);
        }
    }

    fn start_checkout(&mut self, mr_idx: usize) {
        let mr = &self.mrs[mr_idx];
        let info = CheckoutInfo {
            iid: mr.iid,
            title: mr.title.clone(),
            source_branch: mr.source_branch.clone(),
            web_url: mr.web_url.clone(),
        };
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let result = checkout_mr_info(info);
            let _ = tx.send(result);
        });
        self.checkout = Some(CheckoutState {
            mr_idx,
            tick: 0,
            receiver: rx,
        });
        self.set_status("Checking out selected MR...");
    }

    fn open_in_browser(&mut self, mr_idx: usize) {
        let mr = &self.mrs[mr_idx];
        match open::that(&mr.web_url) {
            Ok(()) => self.set_status(&format!("Opening MR !{}: {}", mr.iid, mr.web_url)),
            Err(e) => self.set_status(&format!(
                "Failed to open MR !{} in browser: {} ({})",
                mr.iid, mr.web_url, e
            )),
        }
    }

    fn load_author_mrs(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
        query: &str,
    ) -> TResult<()> {
        let query = query.trim();
        if query.is_empty() {
            return Ok(());
        }

        self.set_status("Loading open merge requests...");
        self.draw(terminal)?;

        let (username, label) =
            if let Some(username) = self.gitlab_client.resolve_author_username(query)? {
                let label = if username.eq_ignore_ascii_case(query) {
                    username.clone()
                } else {
                    format!("{} ({})", query, username)
                };
                (username, label)
            } else {
                (query.to_string(), query.to_string())
            };

        self.mrs =
            self.gitlab_client
                .get_merge_requests(gitlab::client::GetMergeRequestsFilter {
                    author: Some(gitlab::client::MergeRequestFilterAuthor::Username(username)),
                    opened_only: true,
                })?;
        self.visible = (0..self.mrs.len()).collect();
        self.state
            .select(if self.mrs.is_empty() { None } else { Some(0) });
        self.author_label = label;
        if self.mrs.is_empty() {
            self.set_status("No open merge requests found for this author.");
        } else {
            self.status = None;
        }
        Ok(())
    }

    fn selected_mr_idx(&self) -> Option<usize> {
        self.state
            .selected()
            .and_then(|i| self.visible.get(i).copied())
    }

    fn handle_author_input(
        &mut self,
        key_code: ratatui::crossterm::event::KeyCode,
    ) -> TResult<UiAction> {
        let Some(input) = &mut self.input else {
            return Ok(UiAction::None);
        };

        match key_code {
            ratatui::crossterm::event::KeyCode::Esc => {
                self.input = None;
                Ok(UiAction::None)
            }
            ratatui::crossterm::event::KeyCode::Enter => {
                let value = input.value.trim().to_string();
                self.input = None;
                if value.is_empty() {
                    Ok(UiAction::None)
                } else {
                    Ok(UiAction::FilterAuthor(value))
                }
            }
            ratatui::crossterm::event::KeyCode::Backspace => {
                input.value.pop();
                Ok(UiAction::None)
            }
            ratatui::crossterm::event::KeyCode::Char(value) => {
                input.value.push(value);
                Ok(UiAction::None)
            }
            _ => Ok(UiAction::None),
        }
    }

    fn poll_checkout(&mut self) -> TResult<Option<TResult<String>>> {
        if let Some(checkout) = &self.checkout {
            match checkout.receiver.try_recv() {
                Ok(result) => {
                    self.checkout = None;
                    self.status = None;
                    return Ok(Some(result));
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {}
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.checkout = None;
                    self.status = None;
                    return Ok(Some(Err(crate::errors::Error::Generic(
                        "Checkout thread disconnected".to_string(),
                    ))));
                }
            }
        }
        Ok(None)
    }

    fn poll_input_during_checkout(&self) -> TResult<()> {
        if ratatui::crossterm::event::poll(std::time::Duration::from_millis(100)).to_generic()? {
            let _ = ratatui::crossterm::event::read().to_generic()?;
        }
        Ok(())
    }
}

struct CheckoutInfo {
    iid: u64,
    title: String,
    source_branch: String,
    web_url: String,
}

struct CheckoutState {
    mr_idx: usize,
    tick: usize,
    receiver: std::sync::mpsc::Receiver<TResult<String>>,
}

#[derive(Default)]
struct InputState {
    value: String,
}

enum UiAction {
    None,
    Cancel,
    StartCheckout(usize),
    OpenInBrowser(usize),
    FilterAuthor(String),
}

const SPINNER_FRAMES: [&str; 4] = ["|", "/", "-", "\\"];

fn checkout_mr_info(mr: CheckoutInfo) -> TResult<String> {
    GitCommand::new([
        "fetch",
        "origin",
        &format!("{}:{}", &mr.source_branch, &mr.source_branch),
    ])?
    .execute()?;
    GitCommand::new(["checkout", &mr.source_branch])?.execute()?;
    Ok(format!(
        "Checked out to branch `{}` for MR !{} `{}`\nView Merge Request in Browser: {}",
        mr.source_branch, mr.iid, mr.title, mr.web_url
    ))
}
