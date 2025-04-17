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

pub fn delete_account(conn: &Connection, account: &str) -> Result<usize> {
    let result = conn.execute(
        "DELETE FROM passwords WHERE account = ?1",
        params![account],
    )?;

    Ok(result)
}