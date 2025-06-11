use dioxus::prelude::*;
use server::AccountCredentials;
use ui::{Echo, Hero};

use crate::Route;

#[component]
pub fn Home() -> Element {
    let logged_in: bool = true;
    let nav = navigator();

    if logged_in {
        let credentials = AccountCredentials {
            id: 1,
            session_token: [
                0x69, 0x17, 0x97, 0xA4, 0xD1, 0x58, 0xB2, 0xDF, 0x31, 0x2F, 0xAA, 0x60, 0x75, 0xFE,
                0xF3, 0xA6, 0xD5, 0x6F, 0x81, 0xEF, 0xF2, 0x34, 0x82, 0x64, 0x7E, 0x88, 0x4F, 0xFE,
                0xB8, 0x67, 0x2B, 0x78,
            ],
        };
        nav.replace(Route::Contacts { credentials });
    } else {
        nav.replace(Route::RegisterAccount {});
    }

    rsx! {
        Hero {}
        Echo {}
    }
}
