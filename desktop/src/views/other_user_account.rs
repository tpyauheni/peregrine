use client::{future_retry_loop, packet_sender::PacketState};
use dioxus::prelude::*;
use server::{AccountCredentials};
use shared::types::GroupPermissions;

#[component]
pub fn OtherUserAccount(user_id: u64, credentials: AccountCredentials) -> Element {
    let user_data = future_retry_loop!(server::get_user_data(user_id, credentials));
    let user_info = match user_data {
        PacketState::Response(info) => {
            match info {
                Some(info) => {
                    let email = info.email.unwrap_or("Hidden email".to_owned());
                    let username = info.username.unwrap_or("Hidden username".to_owned());
                    rsx! {
                        h4 { margin: 0, "Email: {email}" }
                        h4 { margin: 0, "Username: {username}" }
                        h4 { margin: 0, "Id: {user_id}" }
                    }
                }
                None => rsx!("Removed account"),
            }
        }
        PacketState::Waiting => rsx!("Loading user information..."),
        PacketState::ServerError(err) => rsx!("Server error: {err:?}"),
        PacketState::RequestTimeout => rsx!("Request timeout"),
        PacketState::NotStarted => unreachable!(),
    };
    let joined_groups = future_retry_loop!(server::get_joined_groups(credentials));
    let joined_groups_element = match joined_groups {
        PacketState::Response(groups) => {
            rsx! {
                for group in groups {
                    button {
                        key: group.id,
                        onclick: move |_| async move {
                            match server::send_group_invite(user_id, group.id, GroupPermissions::default().to_bytes(), credentials).await {
                                Ok(invite_id) => {
                                    println!("Sent group invite: {invite_id:?} (for group {} to user {user_id})", group.id);
                                }
                                Err(err) => {
                                    eprintln!("Error from server: {err:?}");
                                }
                            }
                        },
                        {group.name},
                    }
                }
            }
        }
        PacketState::Waiting => rsx!("Loading groups..."),
        PacketState::ServerError(err) => rsx!("Server error: {err:?}"),
        PacketState::RequestTimeout => rsx!("Request timeout"),
        PacketState::NotStarted => unreachable!(),
    };
    rsx! {
        div {
            height: "100%",
            margin: "12px 24px",

            button {
                onclick: |_| {
                    let nav = navigator();
                    nav.go_back();
                },
                "Back"
            }
            h1 { "User" }
            {user_info}
            h2 { "Invite to:" }
            button {
                onclick: move |_| async move {
                    match server::send_dm_invite(user_id, false, credentials).await {
                        Ok(invite_id) => {
                            println!("Sent invite: {invite_id:?}");
                        }
                        Err(err) => {
                            eprintln!("Error from server: {err:?}");
                        }
                    }
                    println!("User {user_id:?} clicked");
                },
                "Direct conversation",
            }
            {joined_groups_element}
        }
    }
}
