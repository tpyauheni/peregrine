use client::future_retry_loop;
use dioxus::{logger::tracing::error, prelude::*};
use server::{AccountCredentials, FoundAccount};

#[derive(Clone, Copy)]
enum Tab {
    SentInvites,
    ReceivedInvites,
}

#[component]
pub fn Invites(credentials: AccountCredentials) -> Element {
    let mut current_tab = use_signal(|| Tab::ReceivedInvites);
    let tab = match *current_tab.read() {
        Tab::SentInvites => rsx!(SentInvitesTab { credentials }),
        Tab::ReceivedInvites => rsx!(ReceivedInvitesTab { credentials }),
    };
    rsx! {
        h1 { "Invites" }
        div {
            display: "flex",
            flex_direction: "row",

            button {
                onclick: move |_| current_tab.set(Tab::ReceivedInvites),
                "Received",
            }
            button {
                onclick: move |_| current_tab.set(Tab::SentInvites),
                "Sent"
            }
            div {
                flex_grow: 1,
                margin: 0,
            }
            button {
                onclick: |_| {
                    let nav = navigator();
                    nav.go_back();
                },
                "Back"
            }
        }
        {tab}
    }
}

#[component]
pub fn SentInvitesTab(credentials: AccountCredentials) -> Element {
    // The following feature is being called every time the tab is switched on purpose. 
    let sent_dm_invites = future_retry_loop!(server::get_sent_dm_invites(credentials));
    rsx! {
        h3 { "Sent invites" }
    }
}

#[component]
pub fn ReceivedInvitesTab(credentials: AccountCredentials) -> Element {
    // The following feature is being called every time the tab is switched on purpose. 
    let received_dm_invites = future_retry_loop!(server::get_received_dm_invites(credentials));
    rsx! {
        h3 { "Received invites" }
    }
}
