use utils::generate_strong_password;
use password_manager_lib::encryption::*;
use password_manager_lib::database::*;
use password_manager_lib::encryption::*;
use base64;
use std::io::{self, Write};
use ratatui::layout::Direction;
use ratatui::widgets::{List, ListItem, ListState};
use ratatui::layout::Rect;
use rusqlite::Connection;
use ratatui::widgets::Wrap;
use ratatui::text::Text;
use ratatui::text::Line;
use crossterm::{
    event::{self, DisableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::error::Error;
use arboard::Clipboard;
use crossterm::cursor::Hide;

pub mod utils;

enum AppState {
    Start,
    Menu,
    Register {
        step: usize,
        username: String,
        password: String,
        password2: String,
        input_buffer: String,
        cursor_pos: usize,
        error_message: Option<String>,
        error_time: Option<std::time::Instant>,
    },
    CreateAccount {
        step: usize,
        account: String,
        username: String,
        password: String,
        input_buffer: String,
        cursor_pos: usize,
    },
    ShowAllVaults {
        entries: Vec<(String, String, String)>,
        scroll: u16,
        selected: usize,
        show_password: bool,
        show_headers: bool,
    },
    ViewVaultDetail {
        account: String,
        username: String,
        password: String,
        previous_entries: Vec<(String, String, String)>,
        previous_scroll: u16,
        previous_selected: usize,
        scroll: u16,
        previous_show_headers: bool,

        email_emoji_pos: Option<(u16, u16)>,
        pass_emoji_pos: Option<(u16, u16)>,

        copy_message: Option<(String, std::time::Instant)>,
        obscure_password: bool,
    },
    EditVault {
        step: usize,
        account: String,
        username: String,
        password: String,
        input_buffer: String,
        old_account: String,
        old_username: String,

        temp_account: String,
        temp_username: String,
        temp_password: String,

        previous_entries: Vec<(String, String, String)>,
        previous_scroll: u16,
        previous_selected: usize,
        previous_show_headers: bool,

        started_editing: bool,

        cursor_pos: usize,
    },
    SearchVault {
        input_buffer: String,
    }
}
// Setting up console environment
fn main() -> Result<(), Box<dyn Error>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();

    execute!(stdout, EnterAlternateScreen, Hide)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let key = [42u8; 32]; // šifrovací kľúč (Key)
    let conn = initialize_db("passwords.db")?;

    let mut state = AppState::Start;

    let result = run_app(&mut terminal, &key, &conn, &mut state);

    disable_raw_mode().ok();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    ).ok();
    terminal.show_cursor().ok();

    if let Err(err) = result {
        eprintln!("Chyba aplikácie: {}", err);
    }

    Ok(())
}

struct MenuState {
    selected: usize,
}

impl MenuState {
    fn new() -> Self {
        Self { selected: 0 }
    }

    fn next(&mut self, max: usize) {
        if self.selected < max - 1 {
            self.selected += 1;
        }
    }

    fn previous(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
    }
}

const START_ITEMS: [&str; 3] = [
    "Login",
    "Register",
    "End"
];
const MENU_ITEMS: [&str; 4] = [
    "Create vault",
    "Show all vaults",
    "Search vault",
    "Logout",
];

fn run_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, key: &[u8; 32], conn: &Connection, state: &mut AppState,) -> Result<(), Box<dyn std::error::Error>> {
    let mut list_state = ListState::default();
    list_state.select(Some(0));
    
    loop {
        terminal.draw(|f| {
            let size = f.size();

            let block = Block::default()
                .title("My Password Manager")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan));

            let items: Vec<ListItem> = match state {
                AppState::Start => START_ITEMS.iter().map(|item| ListItem::new(*item)).collect(),
                AppState::Menu => MENU_ITEMS.iter().map(|item| ListItem::new(*item)).collect(),
                _ => vec![ListItem::new("Currently working in the terminal to the right.")],
            };

            let chunks = Layout::default()
                .direction(Direction::Horizontal)
                .margin(1)
                .constraints([Constraint::Percentage(40), Constraint::Percentage(60)].as_ref())
                .split(f.size());

            let visible_lines = chunks[1].height as usize;

            let list = List::new(items)
                .block(
                    Block::default()
                        .title("Menu")
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(Color::White))
                )
                .style(Style::default().fg(Color::White))
                .highlight_style(
                    Style::default()
                        .fg(Color::Rgb(255, 165, 0))
                        .add_modifier(Modifier::BOLD)
                        .add_modifier(Modifier::REVERSED),
                );

            f.render_stateful_widget(list, chunks[0], &mut list_state);

            //UI
            match state {
                AppState::Start => {
                    let paragraph = Paragraph::new("Pick an option.")
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)))
                        .block(Block::default().title("Action").borders(Borders::ALL));
                    f.render_widget(paragraph, chunks[1]);
                }

                AppState::Register {step, username, password, password2, input_buffer, cursor_pos, error_message, error_time} => {
                    let label = match step {
                        0 => "Enter nickname:",
                        1 => "Enter password:",
                        2 => "Re-enter password",
                        _ => "Finito!",
                    };

                    let cursor_pos = std::cmp::min(*cursor_pos, input_buffer.len());

                    let before = &input_buffer[..cursor_pos];
                    let cursor_char = input_buffer
                        .chars()
                        .nth(cursor_pos)
                        .unwrap_or(' ');

                    let after = if cursor_pos < input_buffer.len() {
                        &input_buffer[cursor_pos + cursor_char.len_utf8()..]
                    } else {
                    ""
                    };

                    let cursor_pos = std::cmp::min(cursor_pos, input_buffer.len());

                    let before = &input_buffer[..cursor_pos];
                    let cursor_char = input_buffer.chars().nth(cursor_pos).unwrap_or(' ');
                    let after = if cursor_pos < input_buffer.len() {
                    &input_buffer[cursor_pos + cursor_char.len_utf8()..]
                    } else {
                        ""
                    };

                    let spans = vec![
                        Span::styled(before, Style::default().fg(Color::White)),
                        Span::styled(
                            cursor_char.to_string(),
                            Style::default()
                                .fg(Color::Rgb(0, 255, 255))
                                .bg(Color::Rgb(255, 60, 60))
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(after, Style::default().fg(Color::White)),
                    ];

                    let lines = vec![
                        Line::from(Span::styled(label, Style::default().fg(Color::Rgb(255, 60, 60)).add_modifier(Modifier::BOLD))),
                        Line::from(spans),
                    ];

                    let paragraph = Paragraph::new(Text::from(lines))
                        .block(Block::default().title("Registering (Cancel/Start - Esc)").borders(Borders::ALL))
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)));

                    f.render_widget(paragraph, chunks[1]);

                    let actual_pos = std::cmp::min(cursor_pos, input_buffer.len());
                    let cursor_x = chunks[1].x + 1 + actual_pos as u16;
                    let cursor_y = chunks[1].y + 2;
                        f.set_cursor(cursor_x, cursor_y);

                    if let Some(msg) = error_message {
                        let error_paragraph = Paragraph::new(Text::from(Line::from(Span::styled(
                            msg.as_str(),
                            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
                        ))))
                            .block(Block::default().borders(Borders::ALL).title("Error"));

                        let error_rect = Rect {
                            x: chunks[1].x,
                            y: chunks[1].y + 4,
                            width: chunks[1].width,
                            height: 3,
                        };

                        f.render_widget(error_paragraph, error_rect);
                    }
                }

                AppState::Menu => {
                    let paragraph = Paragraph::new("Pick option in menu.")
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)))
                        .block(Block::default().title("Action").borders(Borders::ALL));
                    f.render_widget(paragraph, chunks[1]);
                }

                AppState::CreateAccount { step, input_buffer, account, username, password, cursor_pos} => {
                    let label = match step {
                        0 => "Enter website name:",
                        1 => "Enter email/username:",
                        2 => "Enter password: (# - generate safe password)",
                        _ => "Finito!",
                    };

                    let cursor_pos = std::cmp::min(*cursor_pos, input_buffer.len());

                    let before = &input_buffer[..cursor_pos];
                    let cursor_char = input_buffer
                        .chars()
                        .nth(cursor_pos)
                        .unwrap_or(' ');

                    let after = if cursor_pos < input_buffer.len() {
                        &input_buffer[cursor_pos + cursor_char.len_utf8()..]
                    } else {
                        ""
                    };

                    let cursor_pos = std::cmp::min(cursor_pos, input_buffer.len());

                    let before = &input_buffer[..cursor_pos];
                    let cursor_char = input_buffer.chars().nth(cursor_pos).unwrap_or(' ');
                    let after = if cursor_pos < input_buffer.len() {
                        &input_buffer[cursor_pos + cursor_char.len_utf8()..]
                    } else {
                        ""
                    };

                    let spans = vec![
                        Span::styled(before, Style::default().fg(Color::White)),
                        Span::styled(
                            cursor_char.to_string(),
                            Style::default()
                                .fg(Color::Rgb(0, 255, 255))
                                .bg(Color::Rgb(255, 60, 60))
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(after, Style::default().fg(Color::White)),
                    ];

                    let lines = vec![
                        Line::from(Span::styled(label, Style::default().fg(Color::Rgb(255, 60, 60)).add_modifier(Modifier::BOLD))),
                        Line::from(spans),
                    ];

                    let paragraph = Paragraph::new(Text::from(lines))
                        .block(Block::default().title("Adding Vault manually (Cancel/Menu - Esc)").borders(Borders::ALL))
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)));

                    f.render_widget(paragraph, chunks[1]);

                    let actual_pos = std::cmp::min(cursor_pos, input_buffer.len());
                    let cursor_x = chunks[1].x + 1 + actual_pos as u16;
                    let cursor_y = chunks[1].y + 2;
                    f.set_cursor(cursor_x, cursor_y);
                }

                AppState::ViewVaultDetail { account, username, password, scroll, copy_message, obscure_password, ..} => {
                    let display_password = if *obscure_password {
                        "•".repeat(password.chars().count())
                    } else {
                        password.clone()
                    };

                    let labels = vec![
                        ("Website", account.as_str()),
                        ("Email/Username", username.as_str()),
                        ("Password", display_password.as_str()),
                    ];


                    //let mut email_pos = None;
                    //let mut pass_pos = None;
                    let mut current_line = 0;

                    let content: Vec<Line> = labels
                        .iter()
                        .enumerate()
                        .flat_map(|(i, (label, value))| {
                            let mut label_line = vec![Span::styled(
                                *label,
                                Style::default().fg(Color::Rgb(255, 60, 60)).add_modifier(Modifier::BOLD),
                            )];

                            if i == 1 || i == 2 {
                                label_line.push(Span::raw(" "));

                                let copied_now = copy_message.as_ref()
                                    .map(|(msg, time)| {
                                        time.elapsed().as_secs_f32() < 1.4 &&
                                            ((i == 1 && msg.contains("Email")) || (i == 2 && msg.contains("Password")))
                                    })
                                    .unwrap_or(false);

                                let text = if copied_now {
                                    "(Copied Successfully!)"
                                } else if i == 1 {
                                    "📋 (Copy to clipboard - U)"
                                } else {
                                    "📋 (Copy to clipboard - P, Show password - S)"
                                };

                                label_line.push(Span::styled(
                                    text,
                                    Style::default().fg(if copied_now { Color::Rgb(0, 225, 0) } else { Color::White }),
                                ));
                            }

                            current_line += 3;

                            vec![
                                Line::from(label_line),
                                Line::from(Span::styled(*value, Style::default().fg(Color::White))),
                                Line::from(""),
                            ]
                        })
                        .collect();

                    let visible_lines = chunks[1].height.saturating_sub(2) as usize;
                    let max_scroll = content.len().saturating_sub(visible_lines);
                    let actual_scroll = (*scroll as usize).min(max_scroll) as u16;

                    let paragraph = Paragraph::new(Text::from(content))
                        .block(Block::default().title("Vault details (Edit - E, Delete - D, Go back - Esc)").borders(Borders::ALL))
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)))
                        .wrap(Wrap { trim: false });

                    f.render_widget(paragraph.scroll((*scroll, 0)), chunks[1]);
                }


                AppState::ShowAllVaults { entries, scroll, selected, show_password, show_headers } => {
                    let mut lines = vec![];
                    let mut last_letter: Option<char> = None;
                    let mut entry_line_indices = vec![];

                    for (i, (acc, user, enc)) in entries.iter().enumerate() {
                        if acc == "No vaults created yet." {
                            let msg = Span::styled(
                                "No vaults created yet.",
                                Style::default().fg(Color::Rgb(255, 60, 60)).add_modifier(Modifier::BOLD),
                            );
                            lines.push(Line::from(vec![msg]));
                            continue;
                        }

                        let first_letter = acc.chars().next().unwrap_or('?').to_ascii_uppercase();

                        if *show_headers && Some(first_letter) != last_letter {
                            lines.push(Line::from(Span::styled(
                                format!("{}", first_letter),
                                Style::default()
                                    .fg(Color::Rgb(255, 60, 60))
                                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                            )));
                            last_letter = Some(first_letter);
                        }

                        let mut line = if user.is_empty() {
                            acc.clone()
                        } else {
                            format!("{} | {}", acc, user)
                        };

                        if *show_headers && Some(first_letter) != last_letter {
                            lines.push(Line::from(Span::styled(
                                format!("{}", first_letter),
                                Style::default()
                                    .fg(Color::Rgb(255, 60, 60))
                                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
                            )));
                            last_letter = Some(first_letter);
                        }

                        let styled_line = if acc == "Sorry, no results :(" {
                            Span::styled(line, Style::default().fg(Color::Rgb(255, 60, 60)).add_modifier(Modifier::BOLD))
                        } else if i == *selected {
                            Span::styled(line, Style::default().fg(Color::Rgb(255, 165, 0)).add_modifier(Modifier::BOLD))
                        } else {
                            Span::styled(line, Style::default().fg(Color::White))
                        };


                        entry_line_indices.push(lines.len());
                        lines.push(Line::from(vec![styled_line]));
                    }

                    let selected_line = *entry_line_indices.get(*selected).unwrap_or(&0);
                    let visible_lines = chunks[1].height.saturating_sub(2) as usize;

                    let mut scroll_offset = *scroll as usize;

                    if *selected == 0 {
                        scroll_offset = 0;
                    } else if selected_line < scroll_offset {
                        scroll_offset = selected_line;
                    } else if selected_line >= scroll_offset + visible_lines {
                        scroll_offset = selected_line + 1 - visible_lines;
                    }

                    *scroll = scroll_offset as u16;

                    let max_scroll = lines.len().saturating_sub(visible_lines);
                    let start = scroll_offset.min(max_scroll);
                    let end = (start + visible_lines).min(lines.len());

                    let paragraph = Paragraph::new(Text::from(lines[start..end].to_vec()))
                        .style(Style::default().fg(Color::LightCyan))
                        .block(Block::default().title("Vaults (Select - Enter, Scroll, Menu - Esc)").borders(Borders::ALL))
                        .wrap(Wrap { trim: false });

                    f.render_widget(paragraph, chunks[1]);

                }
                
                AppState::SearchVault { input_buffer } => {
                    let lines = vec![
                        Line::from(Span::styled("Enter website name to filter:", Style::default().fg(Color::Rgb(255, 60, 60)).add_modifier(Modifier::BOLD))),
                        Line::from(Span::styled(input_buffer.as_str(), Style::default().fg(Color::White))),
                    ];
                    let paragraph = Paragraph::new(Text::from(lines))
                        .block(Block::default().title("Search Vault").borders(Borders::ALL))
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)));

                    f.render_widget(paragraph, chunks[1]);
                }

                AppState::EditVault { step, input_buffer, account, username, password, cursor_pos, ..} => {
                    let label = match step {
                        0 => "Edit Website (account):",
                        1 => "Edit Email/Username:",
                        2 => "Edit Password: (# - generate safe password)",
                        _ => "Updating...",
                    };

                    let current_value = match step {
                        0 => account,
                        1 => username,
                        2 => password,
                        _ => "",
                    };

                    let display_value = input_buffer;

                    let mut lines = vec![
                        Line::from(Span::styled(
                            label,
                            Style::default()
                                .fg(Color::Rgb(255, 60, 60))
                                .add_modifier(Modifier::BOLD),
                        )),
                    ];

                    let cursor_pos = std::cmp::min(*cursor_pos, display_value.len());

                    let mut spans = vec![];
                    let before = &display_value[..cursor_pos];
                    let cursor_char = display_value
                        .chars()
                        .nth(cursor_pos)
                        .unwrap_or(' ');

                    let after = if cursor_pos < display_value.len() {
                        &display_value[cursor_pos + cursor_char.len_utf8()..]
                    } else {
                        ""
                    };

                    spans.push(Span::styled(before, Style::default().fg(Color::White)));
                    spans.push(Span::styled(
                        cursor_char.to_string(),
                        Style::default()
                            .fg(Color::Rgb(0, 255, 255))
                            .bg(Color::Rgb(255, 60, 60))
                            .add_modifier(Modifier::BOLD),
                    ));
                    spans.push(Span::styled(after, Style::default().fg(Color::White)));

                    lines.push(Line::from(spans));


                    let paragraph = Paragraph::new(Text::from(lines))
                        .block(Block::default().title("Edit Vault (Next - Enter, Cancel - Esc)").borders(Borders::ALL))
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)));

                    f.render_widget(paragraph, chunks[1]);

                    let cursor_x = chunks[1].x + 1 + cursor_pos as u16;
                    let cursor_y = chunks[1].y + 2;
                    f.set_cursor(cursor_x, cursor_y);
                }
            }
        })?;

        //functionality of scenes
        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(KeyEvent { code, kind: KeyEventKind::Press, .. }) = event::read()? {
                let selected = list_state.selected().unwrap_or(0);

                match state {
                    AppState::Start => {
                        match code {
                            KeyCode::Down => {
                                let new_index = (selected + 1).min(MENU_ITEMS.len() - 1);
                                list_state.select(Some(new_index));
                            }
                            KeyCode::Up => {
                                let new_index = selected.saturating_sub(1);
                                list_state.select(Some(new_index));
                            }
                            KeyCode::Enter => match selected {
                                0 => {
                                    *state = AppState::Menu;
                                },
                                1 => {
                                    *state = AppState::Register {
                                        step: 0,
                                        username: String::new(),
                                        password: String::new(),
                                        password2: String::new(),
                                        input_buffer: String::new(),
                                        cursor_pos: 0,
                                        error_message: None,
                                        error_time: None,
                                    };
                                },
                                2 => return Ok(()),
                                _ => {}
                            },
                            KeyCode::Char('q') => return Ok(()),
                            _ => {}
                        }
                    }

                    AppState::Register {step, username, password, password2, input_buffer, cursor_pos, error_message, error_time} => {
                        match code {
                            KeyCode::Char(c) => {
                                if *cursor_pos <= input_buffer.len() {
                                    input_buffer.insert(*cursor_pos, c);
                                    *cursor_pos += 1;
                                }
                            }
                            KeyCode::Backspace => {
                                if *cursor_pos > 0 && *cursor_pos <= input_buffer.len() {
                                    input_buffer.remove(*cursor_pos - 1);
                                    *cursor_pos -= 1;
                                }
                            }
                            KeyCode::Left => {
                                if *cursor_pos > 0 {
                                    *cursor_pos -= 1;
                                }
                            }
                            KeyCode::Right => {
                                if *cursor_pos < input_buffer.len() {
                                    *cursor_pos += 1;
                                }
                            }
                            KeyCode::Enter => {
                                match *step {
                                    0 => {
                                        let mut stmt = conn.prepare("SELECT 1 FROM users WHERE username = ?1")?;
                                        let exists = stmt.exists([input_buffer.as_str()])?;

                                        if exists {
                                            *error_message = Some("Username already taken".to_string());
                                            *error_time = Some(std::time::Instant::now());
                                            input_buffer.clear();
                                            *cursor_pos = 0;
                                        } else {
                                            *username = input_buffer.clone();
                                            input_buffer.clear();
                                            *cursor_pos = 0;
                                            *step = 1;
                                        }
                                    }
                                    1 => {
                                        *password = input_buffer.clone();
                                        input_buffer.clear();
                                        *cursor_pos = 0;
                                        *step = 2;
                                    }
                                    2 => {
                                        *password2 = input_buffer.clone();
                                        if password == password2 {
                                            if let Err(err) = register_user(conn, username, password) {
                                                *error_message = Some(format!("Failed to register: {}", err));
                                                *error_time = Some(std::time::Instant::now());
                                                *step = 0;
                                            } else {
                                                input_buffer.clear();
                                                *cursor_pos = 0;
                                                *state = AppState::Menu;
                                            }
                                        }

                                        else {
                                            *error_message = Some("Passwords do not match".to_string());
                                            *error_time = Some(std::time::Instant::now());
                                            *step = 1;
                                        }
                                    }
                                    _ => {}
                                }
                            }
                            KeyCode::Esc => {
                                input_buffer.clear();
                                *state = AppState::Start;
                            }
                            _ => {}
                        }
                    }

                    AppState::Menu => {
                        match code {
                            KeyCode::Down => {
                                let new_index = (selected + 1).min(MENU_ITEMS.len() - 1);
                                list_state.select(Some(new_index));
                            }
                            KeyCode::Up => {
                                let new_index = selected.saturating_sub(1);
                                list_state.select(Some(new_index));
                            }
                            KeyCode::Enter => match selected {
                                0 => {
                                    *state = AppState::CreateAccount {
                                        step: 0,
                                        account: String::new(),
                                        username: String::new(),
                                        password: String::new(),
                                        input_buffer: String::new(),
                                        cursor_pos: 0,
                                    };
                                }
                                1 => { *state = AppState::SearchVault {
                                    input_buffer: String::new(),}; }
                                2 => {
                                    let mut vaults: Vec<(String, String, String)> = get_passwords(conn)?
                                        .into_iter()
                                        .map(|(acc, user, enc)| (acc, user, base64::encode(enc)))
                                        .collect();

                                    let show_headers = !vaults.is_empty();

                                    if vaults.is_empty() {
                                        vaults.push((
                                            "No vaults created yet.".to_string(),
                                            "".to_string(),
                                            "".to_string()
                                        ));
                                    }

                                    vaults.sort_by(|a, b| {
                                        let site_cmp = a.0.to_lowercase().cmp(&b.0.to_lowercase());
                                        if site_cmp == std::cmp::Ordering::Equal {
                                            a.1.to_lowercase().cmp(&b.1.to_lowercase())
                                        } else {
                                            site_cmp
                                        }
                                    });

                                    *state = AppState::ShowAllVaults {
                                        entries: vaults,
                                        scroll: 0,
                                        selected: 0,
                                        show_password: false,
                                        show_headers,
                                    };
                                }
                                3 => return Ok(()),
                                _ => {}
                            },
                            KeyCode::Char('q') => return Ok(()),
                            _ => {}
                        }
                    }

                    AppState::ShowAllVaults { scroll, selected, show_password, entries, show_headers} => {
                        match code {
                            KeyCode::Esc => {
                                *state = AppState::Menu;
                            }
                            KeyCode::Down => {
                                if *selected < entries.len().saturating_sub(1) {
                                    *selected += 1;

                                    let mut line_index = 0;
                                    let mut last_letter: Option<char> = None;

                                    for (i, (acc, _, _)) in entries.iter().enumerate() {
                                        let first_letter = acc.chars().next().unwrap_or('?').to_ascii_uppercase();
                                        if Some(first_letter) != last_letter {
                                            line_index += 1;
                                            last_letter = Some(first_letter);
                                        }
                                        if i == *selected {
                                            break;
                                        }
                                        line_index += 1;
                                    }

                                    let terminal_height = terminal.size()?.height.saturating_sub(4) as usize;
                                    if line_index >= *scroll as usize + terminal_height {
                                        *scroll = (line_index + 1 - terminal_height) as u16;
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if *selected > 0 {
                                    *selected -= 1;

                                    let mut line_index = 0;
                                    let mut last_letter: Option<char> = None;

                                    for (i, (acc, _, _)) in entries.iter().enumerate() {
                                        let first_letter = acc.chars().next().unwrap_or('?').to_ascii_uppercase();
                                        if Some(first_letter) != last_letter {
                                            line_index += 1;
                                            last_letter = Some(first_letter);
                                        }
                                        if i == *selected {
                                            break;
                                        }
                                        line_index += 1;
                                    }

                                    if line_index < *scroll as usize {
                                        *scroll = line_index as u16;
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                let (acc, user, enc) = &entries[*selected];
                                if acc == "Sorry, no results :(" || acc == "No vaults created yet." {
                                    continue;
                                }
                                let decrypted = decrypt(&base64::decode(enc).unwrap_or_default(), key).unwrap_or("ERR".to_string());

                                *state = AppState::ViewVaultDetail {
                                    account: acc.clone(),
                                    username: user.clone(),
                                    password: decrypted,
                                    previous_entries: entries.clone(),
                                    previous_scroll: *scroll,
                                    previous_selected: *selected,
                                    scroll: 0,
                                    previous_show_headers: *show_headers,
                                    email_emoji_pos: None,
                                    pass_emoji_pos: None,
                                    copy_message: None,
                                    obscure_password: true,
                                };
                            }
                            _ => {}
                        }
                    }

                    AppState::ViewVaultDetail {
                        account,
                        username,
                        password,
                        previous_entries,
                        previous_scroll,
                        previous_selected,
                        scroll,
                        previous_show_headers,
                        obscure_password,
                        ..
                    } => {
                        match code {
                            KeyCode::Esc => {
                                *state = AppState::ShowAllVaults {
                                    entries: previous_entries.clone(),
                                    scroll: *previous_scroll,
                                    selected: *previous_selected,
                                    show_password: false,
                                    show_headers: *previous_show_headers,
                                };
                            }
                            KeyCode::Down => {
                                let content_lines: u16 = 3 * 3;
                                let visible_lines = terminal.size()?.height.saturating_sub(4);

                                let max_scroll = content_lines.saturating_sub(visible_lines);

                                if *scroll < max_scroll {
                                    *scroll += 1;
                                }
                            }
                            KeyCode::Up => {
                                if *scroll > 0 {
                                    *scroll -= 1;
                                }
                            }
                            KeyCode::Char('d') => {
                                delete_vault(conn, account, username)?;
                                let mut new_entries = previous_entries.clone();
                                new_entries.retain(|(a, u, _)| a != account || u != username);

                                if new_entries.is_empty() {
                                    new_entries.push((
                                        "Sorry, no results :(".to_string(),
                                        "".to_string(),
                                        "".to_string(),
                                    ));
                                }
                                
                                *state = AppState::ShowAllVaults {
                                    entries: new_entries,
                                    scroll: *previous_scroll,
                                    selected: 0,
                                    show_password: false,
                                    show_headers: *previous_show_headers,
                                };
                            }
                            KeyCode::Char('e') => {
                                *state = AppState::EditVault {
                                    step: 0,
                                    account: account.clone(),
                                    username: username.clone(),
                                    password: password.clone(),
                                    input_buffer: account.clone(),
                                    old_account: account.clone(),
                                    old_username: username.clone(),
                                    temp_account: account.clone(),
                                    temp_username: username.clone(),
                                    temp_password: password.clone(),
                                    previous_entries: previous_entries.clone(),
                                    previous_scroll: *previous_scroll,
                                    previous_selected: *previous_selected,
                                    previous_show_headers: *previous_show_headers,
                                    started_editing: false,
                                    cursor_pos: account.len(),
                                };
                            }
                            KeyCode::Char('u') => {
                                if let Ok(mut cb) = Clipboard::new() {
                                    cb.set_text(username.clone()).ok();
                                }

                                *state = AppState::ViewVaultDetail {
                                    account: account.clone(),
                                    username: username.clone(),
                                    password: password.clone(),
                                    previous_entries: previous_entries.clone(),
                                    previous_scroll: *previous_scroll,
                                    previous_selected: *previous_selected,
                                    scroll: *scroll,
                                    previous_show_headers: *previous_show_headers,
                                    email_emoji_pos: None,
                                    pass_emoji_pos: None,
                                    copy_message: Some(("Email/Username copied!".to_string(), std::time::Instant::now())),
                                    obscure_password: *obscure_password,
                                };
                            }
                            KeyCode::Char('p') => {
                                if let Ok(mut cb) = Clipboard::new() {
                                    cb.set_text(password.clone()).ok();
                                }

                                *state = AppState::ViewVaultDetail {
                                    account: account.clone(),
                                    username: username.clone(),
                                    password: password.clone(),
                                    previous_entries: previous_entries.clone(),
                                    previous_scroll: *previous_scroll,
                                    previous_selected: *previous_selected,
                                    scroll: *scroll,
                                    previous_show_headers: *previous_show_headers,
                                    email_emoji_pos: None,
                                    pass_emoji_pos: None,
                                    copy_message: Some(("Password copied!".to_string(), std::time::Instant::now())),
                                    obscure_password: *obscure_password,
                                };
                            }
                            KeyCode::Char('s') => {
                                *state = AppState::ViewVaultDetail {
                                    account: account.clone(),
                                    username: username.clone(),
                                    password: password.clone(),
                                    previous_entries: previous_entries.clone(),
                                    previous_scroll: *previous_scroll,
                                    previous_selected: *previous_selected,
                                    scroll: *scroll,
                                    previous_show_headers: *previous_show_headers,
                                    email_emoji_pos: None,
                                    pass_emoji_pos: None,
                                    copy_message: None,
                                    obscure_password: !*obscure_password,
                                };
                            }
                            _ => {}
                        }
                    }

                    AppState::SearchVault { input_buffer } => {
                        match code {
                            KeyCode::Char(c) => input_buffer.push(c),
                            KeyCode::Backspace => { input_buffer.pop(); }
                            KeyCode::Enter => {
                                if input_buffer.trim().is_empty() {
                                    continue;
                                }
                                
                                let filtered: Vec<_> = get_passwords(conn)?
                                    .into_iter()
                                    .filter(|(site, _, _)| site.to_lowercase().contains(&input_buffer.to_lowercase()))
                                    .map(|(acc, user, enc)| (acc, user, base64::encode(enc)))
                                    .collect();

                                let mut entries = if filtered.is_empty() {
                                    vec![("Sorry, no results :(".to_string(), "".to_string(), "".to_string())]
                                } else {
                                    filtered
                                };

                                entries.sort_by(|a, b| {
                                    let site_cmp = a.0.to_lowercase().cmp(&b.0.to_lowercase());
                                    if site_cmp == std::cmp::Ordering::Equal {
                                        a.1.to_lowercase().cmp(&b.1.to_lowercase())
                                    } else {
                                        site_cmp
                                    }
                                });

                                *state = AppState::ShowAllVaults {
                                    entries,
                                    scroll: 0,
                                    selected: 0,
                                    show_password: false,
                                    show_headers: false,
                                };
                            }
                            KeyCode::Esc => {
                                *state = AppState::Menu;
                            }
                            _ => {}
                        }
                    }

                    AppState::CreateAccount {
                        step,
                        input_buffer,
                        account,
                        username,
                        password,
                        cursor_pos,
                    } => {
                        match code {
                            KeyCode::Char(c) if *step == 2 && (c == '#') => {
                                let generated = generate_strong_password(16);
                                *password = generated.clone();
                                *input_buffer = generated;
                                *cursor_pos = input_buffer.len();
                            }
                            KeyCode::Char(c) => {
                                if *cursor_pos <= input_buffer.len() {
                                    input_buffer.insert(*cursor_pos, c);
                                    *cursor_pos += 1;
                                }
                            }
                            KeyCode::Backspace => {
                                if *cursor_pos > 0 && *cursor_pos <= input_buffer.len() {
                                    input_buffer.remove(*cursor_pos - 1);
                                    *cursor_pos -= 1;
                                }
                            }
                            KeyCode::Left => {
                                if *cursor_pos > 0 {
                                    *cursor_pos -= 1;
                                }
                            }
                            KeyCode::Right => {
                                if *cursor_pos < input_buffer.len() {
                                    *cursor_pos += 1;
                                }
                            }
                            KeyCode::Enter => {
                                match *step {
                                    0 => {
                                        *account = input_buffer.clone();
                                        input_buffer.clear();
                                        *cursor_pos = 0;
                                        *step = 1;
                                    }
                                    1 => {
                                        *username = input_buffer.clone();
                                        input_buffer.clear();
                                        *cursor_pos = 0;
                                        *step = 2;
                                    }
                                    2 => {
                                        *password = input_buffer.clone();

                                        let encrypted = encrypt(password, key);
                                        insert_password(conn, account, username, &encrypted)?;
                                        *state = AppState::Menu;
                                    }
                                    _ => {}
                                }
                            }
                            KeyCode::Esc => {
                                *state = AppState::Menu;
                            }
                            _ => {}
                        }
                    }

                    AppState::EditVault {
                        step,
                        account,
                        username,
                        password,
                        input_buffer,
                        old_account,
                        old_username,
                        temp_account,
                        temp_username,
                        temp_password,
                        previous_entries,
                        previous_scroll,
                        previous_selected,
                        previous_show_headers,
                        started_editing,
                        cursor_pos,
                    } => {
                        match code {
                            KeyCode::Char('#') if *step == 2 => {
                                let generated = generate_strong_password(16);
                                *input_buffer = generated.clone();
                                *cursor_pos = generated.len();
                            }
                            KeyCode::Char(c) => {
                                if *cursor_pos <= input_buffer.len() {
                                    input_buffer.insert(*cursor_pos, c);
                                    *cursor_pos += 1;
                                }
                            }
                            KeyCode::Backspace => {
                                if *cursor_pos > 0 && *cursor_pos <= input_buffer.len() {
                                    input_buffer.remove(*cursor_pos - 1);
                                    *cursor_pos -= 1;
                                }
                            }
                            KeyCode::Left => {
                                if *cursor_pos > 0 {
                                    *cursor_pos -= 1;
                                }
                            }
                            KeyCode::Right => {
                                if *cursor_pos < input_buffer.len() {
                                    *cursor_pos += 1;
                                }
                            }
                            KeyCode::Enter => {
                                match *step {
                                    0 => {
                                        *temp_account = input_buffer.clone();
                                        *step = 1;
                                        *input_buffer = username.clone();
                                        *cursor_pos = input_buffer.len();
                                    }
                                    1 => {
                                        *temp_username = input_buffer.clone();
                                        *step = 2;
                                        *input_buffer = password.clone();
                                        *cursor_pos = input_buffer.len();
                                    }
                                    2 => {
                                        *cursor_pos = input_buffer.len();
                                        *temp_password = input_buffer.clone();

                                        *account = temp_account.clone();
                                        *username = temp_username.clone();
                                        *password = temp_password.clone();

                                        let encrypted = encrypt(password, key);
                                        update_vault(conn, old_account, old_username, account, username, encrypted.as_slice())?;

                                        let mut updated_entries: Vec<(String, String, String)> = get_passwords(conn)?
                                            .into_iter()
                                            .map(|(a, u, e)| (a, u, base64::encode(e)))
                                            .collect();

                                        updated_entries.sort_by(|a, b| {
                                            let site_cmp = a.0.to_lowercase().cmp(&b.0.to_lowercase());
                                            if site_cmp == std::cmp::Ordering::Equal {
                                                a.1.to_lowercase().cmp(&b.1.to_lowercase())
                                            } else {
                                                site_cmp
                                            }
                                        });

                                        let selected_index = updated_entries
                                            .iter()
                                            .position(|(a, u, _)| a == account && u == username)
                                            .unwrap_or(0);

                                        *state = AppState::ViewVaultDetail {
                                            account: account.clone(),
                                            username: username.clone(),
                                            password: password.clone(),
                                            previous_entries: updated_entries,
                                            previous_scroll: 0,
                                            previous_selected: selected_index,
                                            scroll: 0,
                                            previous_show_headers: true,
                                            email_emoji_pos: None,
                                            pass_emoji_pos: None,
                                            copy_message: None,
                                            obscure_password: true,
                                        };
                                    }
                                    _ => {}
                                }
                            }
                            KeyCode::Esc => {
                                *state = AppState::ViewVaultDetail {
                                    account: old_account.clone(),
                                    username: old_username.clone(),
                                    password: password.clone(),
                                    previous_entries: previous_entries.clone(),
                                    previous_scroll: *previous_scroll,
                                    previous_selected: *previous_selected,
                                    scroll: 0,
                                    previous_show_headers: *previous_show_headers,
                                    email_emoji_pos: None,
                                    pass_emoji_pos: None,
                                    copy_message: None,
                                    obscure_password: true,
                                };
                            }
                            _ => {}
                        }
                    }
                }

            }

        }


    }

    Ok(())
}
