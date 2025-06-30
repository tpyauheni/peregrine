use dioxus::prelude::*;
use base64::{engine::general_purpose::STANDARD, Engine};

use server::{AccountCredentials};
use client::storage::STORAGE;
use crate::Route;

#[component]
pub fn ChangeCredentials(credentials: AccountCredentials) -> Element {
    let mut session_token = use_signal(|| {
        let mut bytes = vec![];
        bytes.extend(credentials.id.to_le_bytes());
        bytes.extend(credentials.session_token);
        STANDARD.encode(bytes)
    });
    rsx! {
        div {
            height: "100%",
            margin: "12px 24px",

            input {
                value: "{session_token}",
                placeholder: "New session token",
                oninput: move |event| async move {
                    let token = event.value();
                    session_token.set(token);
                },
            }
            button {
                onclick: |_| {
                    let nav = navigator();
                    nav.go_back();
                },
                "Back"
            }
            button {
                onclick: move |_| async move {
                    let session_token = session_token();
                    if session_token.is_empty() {
                        STORAGE.remove_session_credentials();
                    } else {
                        let Ok(bytes) = STANDARD.decode(session_token) else {
                            return;
                        };
                        if bytes.len() != size_of::<u64>() + size_of::<[u8; 32]>() {
                            return;
                        }
                        let credentials = AccountCredentials {
                            id: u64::from_le_bytes(bytes[..8].try_into().unwrap()),
                            session_token: bytes[8..].try_into().unwrap(),
                        };
                        STORAGE.store_session_credentials(credentials);
                    }
                    let nav = navigator();
                    nav.replace(Route::Home {});
                },
                "Change"
            }
        }
    }
}
