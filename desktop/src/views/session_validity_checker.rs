use std::time::Duration;

use dioxus::prelude::*;
use server::AccountCredentials;

use crate::{views::Contacts, Route};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum AuthenticationState {
    ValidCredentials,
    InvalidCredentials,
    Waiting,
    ServerError,
}

pub static WAIT_TIMEOUT: Duration = Duration::from_secs(10);
pub static RETRY_INTERVAL: Duration = Duration::from_secs(3);

#[component]
pub fn SessionValidityChecker(credentials: AccountCredentials) -> Element {
    let mut result = use_signal(|| AuthenticationState::Waiting);
    use_future(move || async move {
        loop {
            let Ok(value) = tokio::time::timeout(
                WAIT_TIMEOUT,
                server::are_session_credentials_valid(credentials)
            ).await else {
                continue;
            };
            let state = match value {
                Ok(value) => {
                    if value {
                        AuthenticationState::ValidCredentials
                    } else {
                        AuthenticationState::InvalidCredentials
                    }
                }
                Err(_err) => {
                    AuthenticationState::ServerError
                }
            };
            result.set(state);
            if state != AuthenticationState::ServerError {
                break;
            }
            tokio::time::sleep(RETRY_INTERVAL).await;
            result.set(AuthenticationState::Waiting);
        }
    });
    let value = *result.read();
    let nav = navigator();
    let state_data = match value {
        AuthenticationState::ValidCredentials => {
            nav.replace(Route::Contacts { credentials });
            rsx! { h3 { "Loading resources" } }
        }
        AuthenticationState::InvalidCredentials => {
            nav.replace(Route::LoginAccount {});
            rsx! { h3 { "Invalid credentials" } }
        }
        AuthenticationState::Waiting => {
            rsx! { h3 { "Checking credentials" } }
        }
        AuthenticationState::ServerError => {
            rsx! { h3 { "Server error" } }
        }
    };
    rsx! {
        h1 { "Loading..." },
        {state_data}
    }
}
