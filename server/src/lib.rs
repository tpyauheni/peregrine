#[cfg(feature = "server")]
mod secret;

use dioxus::{logger::tracing::info, prelude::*};

#[server]
pub async fn create_account(email: String, username: String, password_hash: Vec<u8>) -> Result<Option<String>, ServerFnError> {
    info!("Server request: create account email='{email}', username='{username}', password.len()={}", password_hash.len());
    info!("Secret 1: '{}', secret 2: '{}'", secret::SECRET_SUPERSECRET, secret::secret_supersecret());
    Ok(Some("Not implemented yet".to_owned()))
}

/// Echo the user input on the server.
#[server(Echo)]
pub async fn echo(input: String) -> Result<String, ServerFnError> {
    Ok(input)
}
