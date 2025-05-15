use argon2::password_hash::SaltString;
use rusqlite::{Connection, Result, params};
use crate::crypto;

pub fn initialize_db(path: &str) -> Result<Connection> {
    let conn = Connection::open(path)?;

    conn.execute(
    "CREATE TABLE IF NOT EXISTS users (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        username TEXT UNIQUE NOT NULL,
        password_hash TEXT NOT NULL
        )",
    [],
    )?;

    conn.execute(
        "CREATE TABLE IF NOT EXISTS passwords (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            user_id INTEGER NOT NULL,
            account TEXT NOT NULL,
            username TEXT NOT NULL,
            password_encrypted BLOB NOT NULL,
            FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
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
    user_id: &i64,
) -> Result<()> {
    conn.execute(
        "INSERT INTO passwords (user_id, account, username, password_encrypted) VALUES (?1, ?2, ?3, ?4)",
        params![user_id, account, username, password_encrypted],
    )?;

    Ok(())
}

pub fn get_passwords(conn: &Connection, user_id: &i64) -> Result<Vec<(String, String, Vec<u8>)>> {
    let mut stmt = conn.prepare("SELECT account, username, password_encrypted FROM passwords WHERE user_id = ?1")?;

    let result = stmt
        .query_map(params![user_id], |row| {
            Ok((
                row.get(0)?, // account
                row.get(1)?, // username
                row.get(2)?, // password_encrypted
            ))
        })?
        .collect();

    result
}

pub fn delete_vault(conn: &Connection, account: &str, username: &str, user_id: &i64) -> rusqlite::Result<()> {
    conn.execute(
        "DELETE FROM passwords WHERE account = ?1 AND username = ?2 AND user_id = ?3",
        params![account, username, user_id],
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
    user_id: &i64,
) -> rusqlite::Result<()> {
    conn.execute(
        "UPDATE passwords SET account = ?1, username = ?2, password_encrypted = ?3 WHERE account = ?4 AND username = ?5 AND user_id = ?6",
        params![new_account, new_username, new_encrypted_password, old_account, old_username, user_id],
    )?;
    Ok(())
}

pub fn login_user(conn: &Connection, username: &str, password: &str) -> Option<i64> {
    let mut stmt = conn.prepare("SELECT id, password_hash FROM users WHERE username = ?1").ok()?;
    let mut rows = stmt.query(params![username]).ok()?;

    if let Some(row) = rows.next().ok()? {
        let user_id: i64 = row.get(0).ok()?;
        let hash: String = row.get(1).ok()?;

        if crypto::verify_password(&hash, password) {
            return Some(user_id);
        }
    }
    None
}

pub fn register_user(conn: &Connection, username: &str, password: &str) -> rusqlite::Result<()> {
    let (hash, _salt) = crypto::hash_password(password);
    let hashed = crypto::derive_key_from_password(password, stringify!((_salt)));
    conn.execute(
        "INSERT INTO users (username, password_hash) VALUES (?1, ?2)",
        params![username, hash],
    )?;
    Ok(())
}

pub fn get_user_id(conn: &Connection, username: &str) -> Result<i64> {
    conn.query_row(
        "SELECT id FROM users WHERE username = ?1",
        params![username],
        |row| row.get(0),
    )
}