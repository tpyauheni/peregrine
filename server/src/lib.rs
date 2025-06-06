#[cfg(feature = "server")]
mod secret;

use std::{fmt::Display, str::FromStr};

use base64::{prelude::BASE64_URL_SAFE_NO_PAD, Engine};
use dioxus::{logger::tracing::{info, error, debug}, prelude::*};
use serde::{Deserialize, Serialize};

#[cfg(feature = "server")]
use crate::secret::db::DB;

#[derive(Debug)]
pub enum ServerError {
    CreateAccountDatabaseError,
    CreateSessionDatabaseError,
    FindUserDatabaseError,
    InvalidSessionToken,
}

impl FromStr for ServerError {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "CreateAccountDatabaseError" => Ok(Self::CreateAccountDatabaseError),
            "CreateSessionDatabaseError" => Ok(Self::CreateSessionDatabaseError),
            "FindUserDatabaseError" => Ok(Self::FindUserDatabaseError),
            "InvalidSessionToken" => Ok(Self::InvalidSessionToken),
            _ => Err(()),
        }
    }
}

impl Display for ServerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match *self {
            Self::CreateAccountDatabaseError => "CreateAccountDatabaseError",
            Self::CreateSessionDatabaseError => "CreateSessionDatabaseError",
            Self::FindUserDatabaseError => "FindUserDatabaseError",
            Self::InvalidSessionToken => "InvalidSessionToken",
        })?;
        Ok(())
    }
}

#[server]
pub async fn create_account(
    email: String,
    username: String,
    public_key: Box<[u8]>,
) -> Result<[u8; 32], ServerFnError<ServerError>> {
    match DB.create_account(
        &public_key,
        &[],
        Some(&email),
        if username.is_empty() { None } else { Some(&username) },
    ) {
        Ok(account_id) => {
            info!("New account created: {account_id}");
            match DB.create_session(account_id, None, None) {
                Ok(session_id) => {
                    debug!("New session created: {session_id:?}");
                    Ok(session_id)
                }
                Err(err) => {
                    error!("Failed to create session: {err:?}");
                    Err(
                        ServerFnError::WrappedServerError(
                            ServerError::CreateSessionDatabaseError
                        )
                    )
                }
            }
        }
        Err(err) => {
            error!("Failed to create account: {err:?}");
            Err(
                ServerFnError::WrappedServerError(
                    ServerError::CreateAccountDatabaseError
                )
            )
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Account {
    pub id: u64,
    pub public_key: Box<[u8]>,
    pub encrypted_private_info: Box<[u8]>,
    pub email: Option<String>,
    pub username: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FoundAccount {
    pub id: u64,
    pub username: Option<String>,
    pub email: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DmMessage {
    pub id: u64,
    pub encryption_method: String,
    pub content: Box<[u8]>,
    pub reply_to: Option<u64>,
    pub edit_for: Option<u64>,
    pub sent_time: Option<chrono::NaiveDateTime>,
    pub sent_by_me: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountCredentials {
    pub id: u64,
    pub session_token: [u8; 32],
}

impl FromStr for AccountCredentials {
    type Err = usize;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let bytes = BASE64_URL_SAFE_NO_PAD.decode(s).unwrap_or_default();
        if bytes.len() != 40 {
            return Err(bytes.len());
        }
        let id = u64::from_le_bytes(bytes[..8].try_into().unwrap());
        let session_token: [u8; 32] = bytes[8..].try_into().unwrap();
        Ok(Self {
            id,
            session_token,
        })
    }
}

impl Display for AccountCredentials {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut bytes = vec![];
        bytes.reserve_exact(40);
        bytes.extend(self.id.to_le_bytes());
        bytes.extend(self.session_token);
        f.write_str(&BASE64_URL_SAFE_NO_PAD.encode(bytes))?;
        Ok(())
    }
}

#[cfg(feature = "server")]
fn check_session(credentials: AccountCredentials) -> Result<(), ServerFnError<ServerError>> {
    match secret::db::DB.is_session_valid(credentials.id, credentials.session_token) {
        Ok(is_valid) => {
            if is_valid {
                Ok(())
            } else {
                Err(ServerFnError::WrappedServerError(ServerError::InvalidSessionToken))
            } 
        }
        Err(err) => {
            error!("Failed to check if session is valid: {err:?}");
            Err(ServerFnError::WrappedServerError(ServerError::InvalidSessionToken))
        }
    }
}

#[server]
pub async fn find_user(
    query: String,
    credentials: AccountCredentials,
) -> Result<Vec<FoundAccount>, ServerFnError<ServerError>> {
    check_session(credentials)?;

    match DB.find_user(&query) {
        Ok(result) => {
            let mut found_accounts = vec![];
            found_accounts.reserve_exact(result.len());

            for account in result {
                found_accounts.push(
                    FoundAccount {
                        id: account.id,
                        username: account.username,
                        email: account.email,
                    },
                );
            }

            Ok(found_accounts)
        },
        Err(err) => {
            error!("Failed to find user: {err:?}");
            Err(ServerFnError::WrappedServerError(ServerError::FindUserDatabaseError))
        }
    }
}

#[cfg(feature = "server")]
pub fn init_server() {
    println!("Initializing server");
    if let Err(err) = DB.init() {
        eprintln!("An error was encountered while initializing database: {err:?}");
    } else {
        println!("Database initialized successfully");
    }
    println!("Server inited");
}
