use client::{future_retry_loop, packet_sender::PacketState};
use dioxus::prelude::*;
use ::server::*;

use crate::Route;

#[component]
pub fn SessionValidityChecker(credentials: AccountCredentials) -> Element {
    let nav = navigator();
    let state_data = match future_retry_loop!(are_session_credentials_valid(credentials)) {
        PacketState::Response(true) => {
            nav.replace(Route::Contacts { credentials });
            rsx! { h3 { "Loading resources" } }
        }
        PacketState::Response(false) => {
            nav.replace(Route::LoginAccount {});
            rsx! { h3 { "Invalid credentials" } }
        }
        PacketState::Waiting => {
            rsx! { h3 { "Checking credentials" } }
        }
        PacketState::ServerError(err) => {
            rsx! { h3 { "Server error: {err:?}" } }
        }
        PacketState::RequestTimeout => {
            rsx! { h3 { "Request timeout" } }
        }
        PacketState::NotStarted => unreachable!(),
    };
    rsx! {
        h1 { "Loading..." },
        {state_data}
    }
}
