use client::{future_retry_loop, packet_sender::PacketState, storage::STORAGE};
use dioxus::prelude::*;
use postcard::to_allocvec;
use server::{AccountCredentials, UserAccount};
use shared::{crypto::{self, x3dh}, types::GroupPermissions};

fn generate_encrypted_shared_key(id: u64, user_data: PacketState<Option<UserAccount>>, for_dm: bool) -> Option<Box<[u8]>> {
    let PacketState::Response(Some(user)) = user_data else {
        return None;
    };
    let crypto_alg = crypto::preferred_alogirthm();
    let (private_keys, public_keys) = STORAGE.x3dh_data(crypto_alg);
    let shared_key = crypto::symmetric_genkey(
        crypto_alg,
        crypto::KeyStrength::ExtremelyHigh,
    )?;
    let Ok(encrypted_shared_key) = x3dh::encode_x3dh(
        &shared_key,
        private_keys.ik,
        public_keys.ik,
        user.cryptoidentity.clone(),
    ) else {
        return None;
    };
    let encrypted_shared_key = to_allocvec(&encrypted_shared_key).unwrap().into_boxed_slice();
    if for_dm {
        STORAGE.store_dm_key(id, (crypto_alg, &shared_key));
    } else {
        STORAGE.store_group_key(id, (crypto_alg, &shared_key));
    }
    Some(encrypted_shared_key)
}

#[component]
pub fn OtherUserAccount(user_id: u64, credentials: AccountCredentials) -> Element {
    let user_data = future_retry_loop!(server::get_user_data(user_id, credentials));
    let user_info = match user_data {
        PacketState::Response(ref info) => match info {
            Some(info) => {
                let email = info.email.clone().unwrap_or("Hidden email".to_owned());
                let username = info.username.clone().unwrap_or("Hidden username".to_owned());
                rsx! {
                    h4 { margin: 0, "Email: {email}" }
                    h4 { margin: 0, "Username: {username}" }
                    h4 { margin: 0, "Id: {user_id}" }
                }
            }
            None => rsx!("Removed account"),
        },
        PacketState::Waiting => rsx!("Loading user information..."),
        PacketState::ServerError(ref err) => rsx!("Server error: {err:?}"),
        PacketState::RequestTimeout => rsx!("Request timeout"),
        PacketState::NotStarted => unreachable!(),
    };
    let user_data1 = user_data.clone();
    let user_data2 = user_data.clone();
    let joined_groups = future_retry_loop!(server::get_joined_groups(credentials));
    let joined_groups_element = match joined_groups {
        PacketState::Response(groups) => {
            let mut result = rsx!();
            let user_data = &user_data1.clone();
            for group in groups {
                let user_data = user_data.clone();
                result = rsx! {
                    {result}
                    br {}
                    button {
                        key: group.id,
                        margin_top: "6px",
                        onclick: move |_| {
                            let user_data = user_data.clone();
                            async move {
                                match server::send_group_invite(user_id, group.id, GroupPermissions::default().to_bytes(), credentials, generate_encrypted_shared_key(group.id, user_data.clone(), false)).await {
                                    Ok(invite_id) => {
                                        println!("Sent group invite: {invite_id:?} (for group {} to user {user_id})", group.id);
                                    }
                                    Err(err) => {
                                        eprintln!("Error from server: {err:?}");
                                    }
                                }
                            };
                        },
                        {group.name},
                    }
                };
            }
            result
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
            if matches!(user_data, PacketState::Response(_)) {
                h1 { "User" }
                {user_info}
                h2 { "Invite to:" }
                button {
                    onclick: move |_| {
                        let user_data = &user_data2.clone();
                        async move {
                            match server::send_dm_invite(user_id, generate_encrypted_shared_key(user_id, user_data.clone(), true), credentials).await {
                                Ok(invite_id) => {
                                    println!("Sent invite: {invite_id:?}");
                                }
                                Err(err) => {
                                    eprintln!("Error from server: {err:?}");
                                }
                            }
                            println!("User {user_id:?} clicked");
                        };
                    },
                    "Direct conversation",
                }
                {joined_groups_element}
            } else {
                {user_info}
            }
        }
    }
}
