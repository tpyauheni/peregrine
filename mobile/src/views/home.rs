use client::storage::STORAGE;
use dioxus::prelude::*;

use crate::Route;

#[component]
pub fn Home() -> Element {
    let credentials = STORAGE.load_session_credentials();

    let nav = navigator();

    if let Some(credentials) = credentials {
        nav.replace(Route::SessionValidityChecker { credentials });
    } else {
        nav.replace(Route::RegisterAccount {});
    }

    rsx! {
    }
}
