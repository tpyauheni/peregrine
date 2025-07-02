use client::{cache::CACHE, future_retry_loop, packet_sender::PacketState};
use dioxus::prelude::*;

use server::{AccountCredentials, GroupMember, MultiUserGroup, UserAccount};

#[component]
fn User(account: UserAccount, is_admin: bool) -> Element {
    const ICON_TRANSPARENT: Asset = asset!(
        "/assets/icon_transparent.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 97,
                height: 111,
            })
            .with_format(ImageFormat::Avif)
    );

    let mut title = account
        .username
        .unwrap_or(account.email.clone().unwrap_or("Anonymous".to_owned()));
    if is_admin {
        title += " [Administrator]";
    }
    let email = account.email.unwrap_or("Hidden email".to_owned());
    rsx! {
        div {
            class: "item-panel",

            div {
                margin: "0",
                flex: "0 3 48px",
                max_height: "46px",

                img {
                    src: ICON_TRANSPARENT,
                    margin_right: "24px",
                    width: "46px",
                    max_height: "46px",
                }
            }
            div {
                flex: "1 0 auto",

                h3 {
                    padding: 0,
                    margin: 0,
                    {title.clone()}
                }
                p {
                    padding: 0,
                    margin: 0,
                    margin_top: "6px",
                    {email}
                }
            }
        }
    }
}

#[component]
pub fn Member(member: PacketState<Option<UserAccount>>, group_member: GroupMember) -> Element {
    match member {
        PacketState::Response(Some(user)) => {
            rsx! {
                br {}
                User {
                    key: group_member.user_id,
                    account: user,
                    is_admin: group_member.is_admin,
                }
                // button {
                //     key: group.id,
                //     margin_top: "6px",
                //     onclick: move |_| async move {
                //         match server::send_group_invite(user_id, group.id, GroupPermissions::default().to_bytes(), credentials).await {
                //             Ok(invite_id) => {
                //                 println!("Sent group invite: {invite_id:?} (for group {} to user {user_id})", group.id);
                //             }
                //             Err(err) => {
                //                 eprintln!("Error from server: {err:?}");
                //             }
                //         }
                //     },
                //     "Kick"
                // }
            }
        }
        PacketState::Response(None) => rsx!("Deleted account"),
        PacketState::NotStarted |
            PacketState::Waiting => rsx!("Loading member..."),
        PacketState::ServerError(err) => rsx!("Server error: {err:?}"),
        PacketState::RequestTimeout => rsx!("Request timeout"),
    }
}

#[component]
pub fn GroupMenu(group_id: u64, credentials: AccountCredentials) -> Element {
    let group_data = future_retry_loop!(server::get_group_data(group_id, credentials));
    let group_info = match group_data {
        PacketState::Response(info) => match info {
            Some(info) => {
                let _: MultiUserGroup = info;
                rsx! {
                    h3 { margin: 0, "Group name: {info.name}" },
                    h3 { margin: 0, if info.encrypted { "Encrypted" } else { "Not encrypted" } },
                    h3 { margin: 0, if info.public { "Public" } else { "Private" } },
                    h3 { margin: 0, if info.channel { "Channel" } else { "Not a channel" } },
                }
            }
            None => rsx!("Removed group"),
        },
        PacketState::Waiting => rsx!("Loading group information..."),
        PacketState::ServerError(err) => rsx!("Server error: {err:?}"),
        PacketState::RequestTimeout => rsx!("Request timeout"),
        PacketState::NotStarted => unreachable!(),
    };
    let mut cached_members = use_signal(Vec::new);
    let mut cached_members_data = use_signal(Vec::new);
    let group_members = future_retry_loop!(server::get_group_members(group_id, credentials));
    let group_members_element = match group_members {
        PacketState::Response(members) => {
            use_effect(move || {
                cached_members.set(members.clone());
                cached_members_data.set(vec![PacketState::NotStarted; members.len()]);
            });
            let data = cached_members_data();
            if data.len() == cached_members().len() && !data.is_empty() {
                use_future(move || async move {
                    for (i, member) in cached_members().iter().enumerate() {
                        println!("LOADING MEMBER {i}");
                        CACHE.user_data_vec(member.user_id, credentials, &mut cached_members_data, i).await;
                        println!("RESULT: {:?}", cached_members_data()[i]);
                    }
                });
                rsx! {
                    for (i, member) in data.iter().enumerate() {
                        Member { member: member.clone(), group_member: cached_members()[i].clone() }
                    }
                }

            } else {
                rsx!("Loading members...")
            }
        }
        PacketState::Waiting => rsx!("Loading members..."),
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
            h1 { "Group" }
            {group_info}
            h2 {
                margin_bottom: 0,
                "Members:"
            }
            // button {
            //     onclick: move |_| async move {
            //         match server::send_dm_invite(user_id, false, credentials).await {
            //             Ok(invite_id) => {
            //                 println!("Sent invite: {invite_id:?}");
            //             }
            //             Err(err) => {
            //                 eprintln!("Error from server: {err:?}");
            //             }
            //         }
            //         println!("User {user_id:?} clicked");
            //     },
            //     "Direct conversation",
            // }
            {group_members_element}
            br {}
            button {
                onclick: move |_| async move {
                    println!("Leave group pressed!");
                },
                "Leave"
            }
        }
    }
}
