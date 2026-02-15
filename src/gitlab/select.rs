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

    if mrs.is_empty() {
        ratatui::restore();
        return Ok("No open merge requests found for your user.".to_string());
    }

    let result = RatatuiApp::new(mrs).run(&mut terminal);
    ratatui::restore();
    result
}

struct RatatuiApp {
    mrs: Vec<gitlab::client::MergeRequest>,
    visible: Vec<usize>,
    state: ratatui::widgets::TableState,
    status: Option<String>,
    checkout: Option<CheckoutState>,
}

impl RatatuiApp {
    fn new(mrs: Vec<gitlab::client::MergeRequest>) -> Self {
        let visible: Vec<usize> = (0..mrs.len()).collect();
        let mut state = ratatui::widgets::TableState::default();
        state.select(Some(0));
        Self {
            mrs,
            visible,
            state,
            status: None,
            checkout: None,
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
                        .title("Merge Requests"),
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
        match &self.status {
            Some(status) => status.clone(),
            None => "Up/Down or j/k to move, Enter to checkout, q/Esc to cancel".to_string(),
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

enum UiAction {
    None,
    Cancel,
    StartCheckout(usize),
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

fn draw_table(
    terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    mrs: &[gitlab::client::MergeRequest],
    state: &mut ratatui::widgets::TableState,
) -> TResult<()> {
    terminal
        .draw(|frame| {
            let rows = mrs.iter().map(|mr| {
                ratatui::widgets::Row::new(vec![
                    mr.iid.to_string(),
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
                    .title("Merge Requests"),
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

            frame.render_stateful_widget(table, chunks[0], state);
            let help = ratatui::widgets::Paragraph::new(
                "Up/Down or j/k to move, Enter to checkout, q/Esc to cancel",
            );
            frame.render_widget(help, chunks[1]);
        })
        .to_generic()?;
    Ok(())
}

fn handle_input(
    total: usize,
    state: &mut ratatui::widgets::TableState,
) -> TResult<Option<Option<usize>>> {
    match ratatui::crossterm::event::read().to_generic()? {
        ratatui::crossterm::event::Event::Key(key) => {
            if key.kind != ratatui::crossterm::event::KeyEventKind::Press {
                return Ok(None);
            }
            match key.code {
                ratatui::crossterm::event::KeyCode::Char('q')
                | ratatui::crossterm::event::KeyCode::Esc => Ok(Some(None)),
                ratatui::crossterm::event::KeyCode::Down
                | ratatui::crossterm::event::KeyCode::Char('j') => {
                    let idx = state.selected().unwrap_or(0);
                    let next = if idx + 1 >= total { 0 } else { idx + 1 };
                    state.select(Some(next));
                    Ok(None)
                }
                ratatui::crossterm::event::KeyCode::Up
                | ratatui::crossterm::event::KeyCode::Char('k') => {
                    let idx = state.selected().unwrap_or(0);
                    let next = if idx == 0 { total - 1 } else { idx - 1 };
                    state.select(Some(next));
                    Ok(None)
                }
                ratatui::crossterm::event::KeyCode::Enter => Ok(Some(state.selected())),
                _ => Ok(None),
            }
        }
        _ => Ok(None),
    }
}
