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



fn read_input(prompt: &str) -> String {
    print!("{}", prompt);
    io::stdout().flush().unwrap();
    let mut input = String::new();
    io::stdin().read_line(&mut input).expect("Nešlo prečítať vstup");
    input.trim().to_string()
}


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
        input_buffer: String, // <- tu
    },
    ShowAllVaults {
        entries: Vec<(String, String, String)>, // alebo prispôsob podľa tvojej DB
        scroll: u16, // <--- toto
        selected: usize,           // <--- pridaj toto
        show_password: bool,
    },

}



fn main() -> Result<(), Box<dyn Error>> {
    // Terminál setup
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let key = [42u8; 32]; // šifrovací kľúč
    let conn = initialize_db("passwords.db")?; // databáza

    let mut state = AppState::Menu;
    let res = run_app(&mut terminal, &key, &conn, &mut state);



    // Čistenie terminálu po skončení
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("Chyba aplikácie: {}", err);
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

                    let paragraph = Paragraph::new(format!("{}\n{}", label, input_buffer))
                        .style(Style::default().fg(Color::Rgb(0, 255, 255)))
                        .block(Block::default().title("Adding Vault manually").borders(Borders::ALL));
                    f.render_widget(paragraph, chunks[1]);
                }

                AppState::ShowAllVaults { entries, scroll, selected, show_password } => {
                    let mut lines = vec![];

                    for (i, (acc, user, enc)) in entries.iter().enumerate().skip(*scroll as usize) {
                        let mut line = format!("{} | {}", acc, user);
                        if *show_password && i == *selected {
                            let encrypted_bytes = base64::decode(enc).unwrap_or_default();
                            let decrypted = decrypt(&encrypted_bytes, key).unwrap_or("ERR".to_string());
                            line += &format!(" | {}", decrypted);
                        }

                        // Zvýrazni vybraný riadok
                        let styled_line = if i == *selected {
                            Span::styled(line, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
                        } else {
                            Span::raw(line)
                        };

                        lines.push(Line::from(vec![styled_line]));
                    }

                    let paragraph = Paragraph::new(Text::from(lines))
                        .style(Style::default().fg(Color::LightCyan))
                        .block(Block::default().title("Vaulty").borders(Borders::ALL))
                        .wrap(Wrap { trim: false });

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
                                1 => { /* vyhľadať specifiv */ }
                                2 => {
                                    let vaults = get_passwords(conn)?
                                        .into_iter()
                                        .map(|(acc, user, enc)| (acc, user, base64::encode(enc)))
                                        .collect();

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

                    AppState::ShowAllVaults { scroll, selected, show_password, entries } => {
                        match code {
                            KeyCode::Esc => {
                                *state = AppState::Menu;
                            }
                            KeyCode::Down => {
                                if *selected < entries.len().saturating_sub(1) {
                                    *selected += 1;
                                    if *selected as u16 >= *scroll + 10 {
                                        *scroll += 1;
                                    }
                                }
                            }
                            KeyCode::Up => {
                                if *selected > 0 {
                                    *selected -= 1;
                                    if *selected as u16 <= *scroll && *scroll > 0 {
                                        *scroll -= 1;
                                    }
                                }
                            }
                            KeyCode::Enter => {
                                *show_password = !*show_password; // toggle zobrazovania hesla
                            }
                            _ => {}
                        }
                    }


                    AppState::ShowAllVaults { .. } => {
                        match code {
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
