use password_manager_lib::crypto::*;
use password_manager_lib::encryption::*;
use password_manager_lib::database::*;
use password_manager_lib::encryption::*;
use base64;
use std::io::{self, Write};
use ratatui::layout::Direction;
use ratatui::widgets::{List, ListItem, ListState};
use std::{thread, time::Duration};
use rusqlite::Connection;
use ratatui::widgets::Wrap;
use ratatui::text::Text;
use ratatui::text::Line;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEvent, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    text::Span,
    widgets::{Block, Borders, Paragraph},
    Terminal,
};
use std::error::Error;

enum AppState {
    Menu,
    CreateAccount {
        step: usize,
        account: String,
        username: String,
        password: String,
        input_buffer: String,
    },
    ShowAllVaults {
        entries: Vec<(String, String, String)>,
        scroll: u16,
        selected: usize,
        show_password: bool,
    },
    ViewVaultDetail {
        account: String,
        username: String,
        password: String,
        previous_entries: Vec<(String, String, String)>,
        previous_scroll: u16,
        previous_selected: usize,
        scroll: u16,
    },
    SearchVault {
        input_buffer: String,
    }
}


fn main() -> Result<(), Box<dyn Error>> {
    // Terminál setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let key = [42u8; 32]; // šifrovací kľúč
    let conn = initialize_db("passwords.db")?; // databáza

    let mut state = AppState::Menu;

    let result = run_app(&mut terminal, &key, &conn, &mut state);

    // Vždy sa pokús o správne čistenie terminálu
    disable_raw_mode().ok();
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    ).ok();
    terminal.show_cursor().ok();

    // Až potom vypíš chybu ak existuje
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

const MENU_ITEMS: [&str; 5] = ["Create vault", "Search specific vault", "Show all vaults", "Delete vault", "End"];


fn run_app(terminal: &mut Terminal<CrosstermBackend<std::io::Stdout>>, key: &[u8; 32], conn: &Connection, state: &mut AppState,) -> Result<(), Box<dyn std::error::Error>> {
    let mut list_state = ListState::default();
    list_state.select(Some(0)); // výber prvej položky (index 0)



    loop {
        terminal.draw(|f| {
            let size = f.size();

            let block = Block::default()
                .title("My Password Manager")
                .borders(Borders::ALL)
                .style(Style::default().fg(Color::Cyan));

            let items: Vec<ListItem> = MENU_ITEMS
                .iter()
                .map(|item| ListItem::new(*item))
                .collect();

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

            match state {
                AppState::Menu => {
                    let paragraph = Paragraph::new("Pick option in menu.")
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)))
                        .block(Block::default().title("Action").borders(Borders::ALL));
                    f.render_widget(paragraph, chunks[1]);
                }

                AppState::CreateAccount { step, input_buffer, account, username, password } => {
                    let label = match step {
                        0 => "Enter website name:",
                        1 => "Enter email/username:",
                        2 => "Enter password:",
                        _ => "Finito!",
                    };

                    let lines = vec![
                        Line::from(Span::styled(label, Style::default().fg(Color::Rgb(255, 60, 60)))),
                        Line::from(Span::styled(input_buffer.as_str(), Style::default().fg(Color::White))),
                    ];

                    let paragraph = Paragraph::new(Text::from(lines))
                        .block(Block::default().title("Adding Vault manually").borders(Borders::ALL))
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)));
                    
                    f.render_widget(paragraph, chunks[1]);
                }

                AppState::ViewVaultDetail { account, username, password, scroll, ..} => {
                    let labels = vec![
                        ("Website", account),
                        ("Email/Username", username),
                        ("Password", password),
                    ];

                    let content: Vec<Line> = labels
                        .iter()
                        .flat_map(|(label, value)| {
                            vec![
                                Line::from(Span::styled(*label, Style::default().fg(Color::Rgb(255, 60, 60)).add_modifier(Modifier::BOLD))),
                                Line::from(Span::styled(value.as_str(),Style::default().fg(Color::White),)),
                                Line::from(""),
                            ]
                        })
                        .collect();

                    let visible_lines = chunks[1].height.saturating_sub(2) as usize;
                    let max_scroll = content.len().saturating_sub(visible_lines);
                    let actual_scroll = (*scroll as usize).min(max_scroll) as u16;
                    
                    let paragraph = Paragraph::new(Text::from(content))
                        .block(Block::default().title("Vault details (Go back - Esc)").borders(Borders::ALL))
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)))
                        .wrap(Wrap { trim: false });

                    f.render_widget(paragraph.scroll((*scroll, 0)), chunks[1]);
                }


                AppState::ShowAllVaults { entries, scroll, selected, show_password } => {
                    let mut lines = vec![];
                    let visible_lines = chunks[1].height.saturating_sub(2) as usize;


                    for (i, (acc, user, enc)) in entries.iter().enumerate().skip(*scroll as usize).take(visible_lines) {
                        let mut line = if user.is_empty() {
                            acc.clone() // zobraz len prvý stĺpec, bez ' | '
                        } else {
                            format!("{} | {}", acc, user)
                        };
                        
                        if *show_password && i == *selected {
                            let encrypted_bytes = base64::decode(enc).unwrap_or_default();
                            let decrypted = decrypt(&encrypted_bytes, key).unwrap_or("ERR".to_string());
                            line += &format!(" | {}", decrypted);
                        }

                        let styled_line = if acc == "Sorry, no results :(" {
                            Span::styled(line, Style::default().fg(Color::Rgb(255, 60, 60)).add_modifier(Modifier::BOLD))
                        } else if i == *selected {
                            Span::styled(line, Style::default().fg(Color::Rgb(255, 165, 0)).add_modifier(Modifier::BOLD))
                        } else {
                            Span::styled(line, Style::default().fg(Color::White))
                        };


                        lines.push(Line::from(vec![styled_line]));
                    }

                    let paragraph = Paragraph::new(Text::from(lines))
                        .style(Style::default().fg(Color::LightCyan))
                        .block(Block::default().title("Vaults (Select - Enter, Scroll, Menu - Esc)").borders(Borders::ALL))
                        .wrap(Wrap { trim: false });

                    f.render_widget(paragraph, chunks[1]);
                }

                AppState::SearchVault { input_buffer } => {
                    let lines = vec![
                        Line::from(Span::styled("Enter website name to filter:", Style::default().fg(Color::Rgb(255, 60, 60)))),
                        Line::from(Span::styled(input_buffer.as_str(), Style::default().fg(Color::White))),
                    ];
                    let paragraph = Paragraph::new(Text::from(lines))
                        .block(Block::default().title("Search Vault").borders(Borders::ALL))
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)));

                    f.render_widget(paragraph, chunks[1]);
                }


            }
        })?;

        if event::poll(std::time::Duration::from_millis(200))? {
            if let Event::Key(KeyEvent { code, kind: KeyEventKind::Press, .. }) = event::read()? {
                let selected = list_state.selected().unwrap_or(0);

                match state {
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
                                    };
                                }
                                1 => { *state = AppState::SearchVault {
                                    input_buffer: String::new(),}; }
                                2 => {
                                    let mut vaults: Vec<(String, String, String)> = get_passwords(conn)?
                                        .into_iter()
                                        .map(|(acc, user, enc)| (acc, user, base64::encode(enc)))
                                        .collect();
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
                                    };
                                }
                                3 => { /* vymazať */ }
                                4 => return Ok(()),
                                _ => {}
                            },
                            KeyCode::Char('q') => return Ok(()),
                            _ => {}
                        }
                    }

                    AppState::ShowAllVaults { scroll, selected, show_password, entries} => {
                        match code {
                            KeyCode::Esc => {
                                *state = AppState::Menu;
                            }
                            KeyCode::Down => {
                                let size = terminal.size()?; // získaj aktuálnu veľkosť
                                let visible_lines = size.height.saturating_sub(4) as usize; // -4: okraje + padding

                                if *selected < entries.len().saturating_sub(1) {
                                    *selected += 1;
                                    if *selected >= *scroll as usize + visible_lines {
                                        *scroll += 1;
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if *selected > 0 {
                                    *selected -= 1;
                                    if *selected < *scroll as usize && *scroll > 0 {
                                        *scroll -= 1;
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                let (acc, user, enc) = &entries[*selected];
                                if acc == "Sorry, no results :(" {
                                    // nič – nedovoľ vstup
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
                                };
                            }
                            _ => {}
                        }
                    }

                    AppState::ViewVaultDetail {
                        previous_entries,
                        previous_scroll,
                        previous_selected,
                        scroll,
                        ..
                    } => {
                        match code {
                            KeyCode::Esc => {
                                *state = AppState::ShowAllVaults {
                                    entries: previous_entries.clone(),
                                    scroll: *previous_scroll,
                                    selected: *previous_selected,
                                    show_password: false,
                                };
                            }
                            KeyCode::Down => {
                                let content_lines: u16 = 3 * 3; // 3 labely * 3 riadky (label, hodnota, prázdny)
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
                            _ => {}
                        }
                    }

                    AppState::SearchVault { input_buffer } => {
                        match code {
                            KeyCode::Char(c) => input_buffer.push(c),
                            KeyCode::Backspace => { input_buffer.pop(); }
                            KeyCode::Enter => {
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
                        password
                    } => {
                        match code {
                            KeyCode::Char(c) => input_buffer.push(c),
                            KeyCode::Backspace => { input_buffer.pop(); }
                            KeyCode::Enter => {
                                match *step {
                                    0 => {
                                        *account = input_buffer.clone();
                                        input_buffer.clear();
                                        *step = 1;
                                    }
                                    1 => {
                                        *username = input_buffer.clone();
                                        input_buffer.clear();
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
                }

            }

        }


    }

    Ok(())
}
