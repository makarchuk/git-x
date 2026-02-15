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
}

impl RatatuiApp {
    fn new(mrs: Vec<gitlab::client::MergeRequest>) -> Self {
        let visible: Vec<usize> = (0..mrs.len()).collect();
        let mut state = ratatui::widgets::TableState::default();
        state.select(Some(0));
        Self { mrs, visible, state }
    }

    fn run(
        &mut self,
        terminal: &mut ratatui::Terminal<ratatui::backend::CrosstermBackend<std::io::Stdout>>,
    ) -> TResult<String> {
        let selection = loop {
            self.draw(terminal)?;
            if let Some(selection) = self.handle_input()? {
                break selection;
            }
        };

        match selection {
            Some(idx) => checkout_mr(&self.mrs[idx]),
            None => Ok("Selection cancelled.".to_string()),
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

                frame.render_stateful_widget(table, chunks[0], &mut self.state);
                let help = ratatui::widgets::Paragraph::new(
                    "Up/Down or j/k to move, Enter to checkout, q/Esc to cancel",
                );
                frame.render_widget(help, chunks[1]);
            })
            .to_generic()?;
        Ok(())
    }

    fn handle_input(&mut self) -> TResult<Option<Option<usize>>> {
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
                        self.move_selection(1);
                        Ok(None)
                    }
                    ratatui::crossterm::event::KeyCode::Up
                    | ratatui::crossterm::event::KeyCode::Char('k') => {
                        self.move_selection(-1);
                        Ok(None)
                    }
                    ratatui::crossterm::event::KeyCode::Enter => {
                        let idx = self
                            .state
                            .selected()
                            .and_then(|i| self.visible.get(i).copied());
                        Ok(Some(idx))
                    }
                    _ => Ok(None),
                }
            }
            _ => Ok(None),
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
}

fn checkout_mr(mr: &gitlab::client::MergeRequest) -> TResult<String> {
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
