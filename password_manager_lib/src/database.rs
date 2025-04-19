use rusqlite::{Connection, Result, params};

pub fn initialize_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS passwords (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            account TEXT NOT NULL,
            username TEXT NOT NULL,
            password_encrypted BLOB NOT NULL
        )",
        [],
    )?;

    Ok(conn)
}

pub fn insert_password(
    conn: &Connection,
    account: &str,
    username: &str,
    password_encrypted: &[u8],
) -> Result<()> {
    conn.execute(
        "INSERT INTO passwords (account, username, password_encrypted) VALUES (?1, ?2, ?3)",
        params![account, username, password_encrypted],
    )?;

    Ok(())
}

pub fn get_passwords(conn: &Connection) -> Result<Vec<(String, String, Vec<u8>)>> {
    let mut stmt = conn.prepare("SELECT account, username, password_encrypted FROM passwords")?;

    let result = stmt
        .query_map([], |row| {
            Ok((
                row.get(0)?, // account
                row.get(1)?, // username
                row.get(2)?, // password_encrypted
            ))
        })?
        .collect();

    result
}

pub fn get_passwords_by_account(
    conn: &Connection,
    account: &str,
) -> Result<Vec<(String, String, Vec<u8>)>> {
    let mut stmt = conn.prepare(
        "SELECT account, username, password_encrypted FROM passwords WHERE account = ?1",
    )?;

    let rows = stmt
        .query_map(params![account], |row| {
            Ok((
                row.get(0)?, // account
                row.get(1)?, // username
                row.get(2)?, // password_encrypted
            ))
        })?
        .collect();

    rows
}

pub fn delete_vault(conn: &Connection, account: &str, username: &str) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM passwords WHERE account = ?1 AND username = ?2",
        &[account, username],
    )?;
    Ok(())
}

pub fn update_vault(
    conn: &Connection,
    old_account: &str,
    old_username: &str,
    new_account: &str,
    new_username: &str,
    new_encrypted_password: &[u8],
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE passwords SET account = ?1, username = ?2, password_encrypted = ?3 WHERE account = ?4 AND username = ?5",
        params![new_account, new_username, new_encrypted_password, old_account, old_username],
    )?;
    Ok(())
}
