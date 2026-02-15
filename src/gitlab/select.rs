use crate::errors::TResult;
use crate::git::cmd::GitCommand;
use crate::gitlab;

pub fn execute_select(ctx: &crate::gitlab::GitContext) -> TResult<String> {
    let mut terminal = ratatui::init();

    let mut mrs = ctx
        .gitlab_client
        .get_merge_requests(gitlab::client::GetMergeRequestsFilter {
            author: Some(gitlab::client::MergeRequestFilterAuthor::Me),
            opened_only: true,
        })?;

    loop {
        terminal
            .draw(|frame| {
                let mut table = ratatui::widgets::Table::new(
                    mrs.iter().map(|mr| {
                        ratatui::widgets::Row::new(vec![
                            mr.iid.to_string(),
                            mr.title.clone(),
                            mr.state.as_str().to_string(),
                        ])
                    }),
                    vec![1, 20, 3],
                );

                draw(frame, table);
            })
            .unwrap();
    }

    unimplemented!()
}

fn draw(frame: &mut ratatui::Frame, list: ratatui::widgets::Table) {
    frame.render_widget(list, frame.area());
}
