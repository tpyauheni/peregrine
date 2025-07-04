use client::{
    cache::CACHE,
    future_retry_loop,
    packet_sender::{PacketSender, PacketState},
};
use dioxus::prelude::*;
use dioxus_free_icons::icons::go_icons::{
    GoAlert, GoCircleSlash, GoLock, GoPeople, GoSync, GoUnlock,
};
use server::{AccountCredentials, DmInvite, GroupInvite};

#[derive(Clone, Copy)]
enum Tab {
    SentInvites,
    ReceivedInvites,
}

#[component]
#[allow(non_snake_case)]
pub fn Invites(credentials: AccountCredentials) -> Element {
    let mut current_tab = use_signal(|| Tab::ReceivedInvites);
    let tab = match *current_tab.read() {
        Tab::SentInvites => rsx!(SentInvitesTab { credentials }),
        Tab::ReceivedInvites => rsx!(ReceivedInvitesTab { credentials }),
    };
    rsx! {
        div {
            height: "100%",
            margin: "12px 24px",
            h1 { "Invites" }
            div {
                display: "flex",
                flex_direction: "row",

                button {
                    onclick: move |_| current_tab.set(Tab::ReceivedInvites),
                    margin_right: "8px",
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
}

#[derive(Clone, PartialEq)]
enum Invite {
    Conversation(DmInvite),
    Group(GroupInvite),
}

#[component]
#[allow(non_snake_case)]
pub fn SentInvitesTab(credentials: AccountCredentials) -> Element {
    // TODO: Add invite caching so "Loading invites..." won't be shown every time user switches
    // tab. But still make a request each time.
    // The following feature is being called every time the tab is switched on purpose.
    let sent_dm_invites = future_retry_loop!(server::get_sent_dm_invites(credentials));
    let sent_group_invites = future_retry_loop!(server::get_sent_group_invites(credentials));
    let invites = match sent_dm_invites {
        PacketState::Response(dm_invites) => match sent_group_invites {
            PacketState::Response(group_invites) => {
                rsx! {
                    for invite in dm_invites {
                        SentInvite { key: {invite.id * 2}, invite: Invite::Conversation(invite), credentials }
                    }
                    for invite in group_invites {
                        SentInvite { key: {invite.id * 2 + 1}, invite: Invite::Group(invite.clone()), credentials }
                    }
                }
            }
            PacketState::Waiting => rsx!(p { "Loading invites..." }),
            PacketState::ServerError(err) => rsx!(p { "Server error: {err:?}" }),
            PacketState::RequestTimeout => rsx!(p { "Request timeout" }),
            PacketState::NotStarted => unreachable!(),
        },
        PacketState::Waiting => rsx!(p { "Loading invites..." }),
        PacketState::ServerError(err) => rsx!(p { "Server error: {err:?}" }),
        PacketState::RequestTimeout => rsx!(p { "Request timeout" }),
        PacketState::NotStarted => unreachable!(),
    };
    rsx! {
        h3 { "Sent invites" }
        {invites}
    }
}

#[component]
#[allow(non_snake_case)]
pub fn ReceivedInvitesTab(credentials: AccountCredentials) -> Element {
    // The following feature is being called every time the tab is switched on purpose.
    let received_dm_invites = future_retry_loop!(server::get_received_dm_invites(credentials));
    let received_group_invites =
        future_retry_loop!(server::get_received_group_invites(credentials));
    let invites = match received_dm_invites {
        PacketState::Response(dm_invites) => match received_group_invites {
            PacketState::Response(group_invites) => {
                rsx! {
                    for invite in dm_invites {
                        ReceivedInvite { key: {invite.id * 2}, invite: Invite::Conversation(invite), credentials }
                    }
                    for invite in group_invites {
                        ReceivedInvite { key: {invite.id * 2 + 1}, invite: Invite::Group(invite.clone()), credentials }
                    }
                }
            }
            PacketState::Waiting => rsx!(p { "Loading invites..." }),
            PacketState::ServerError(err) => rsx!(p { "Server error: {err:?}" }),
            PacketState::RequestTimeout => rsx!(p { "Request timeout" }),
            PacketState::NotStarted => unreachable!(),
        },
        PacketState::Waiting => rsx!(p { "Loading invites..." }),
        PacketState::ServerError(err) => rsx!(p { "Server error: {err:?}" }),
        PacketState::RequestTimeout => rsx!(p { "Request timeout" }),
        PacketState::NotStarted => unreachable!(),
    };
    rsx! {
        h3 { "Received invites" }
        {invites}
    }
}

#[component]
#[allow(non_snake_case)]
fn SentInvite(invite: Invite, credentials: AccountCredentials) -> Element {
    const ICON_TRANSPARENT: Asset = asset!(
        "/assets/icon_transparent.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 97,
                height: 111,
            })
            .with_format(ImageFormat::Avif)
    );

    let mut cancel_result = use_signal(|| PacketState::NotStarted);
    let mut user_data = use_signal(|| PacketState::NotStarted);
    let mut group_data = use_signal(|| PacketState::NotStarted);
    let status = match (*cancel_result.read()).clone() {
        PacketState::Response(()) => {
            return rsx!();
        }
        PacketState::Waiting => rsx!(p { "Rejecting..." }),
        PacketState::ServerError(err) => rsx!(p { "Server error: {err:?}" }),
        PacketState::RequestTimeout => rsx!(p { "Request timed out" }),
        PacketState::NotStarted => rsx!(),
    };

    macro_rules! icon {
        ($icon:expr) => {
            rsx! {
                dioxus_free_icons::Icon {
                    width: 16,
                    height: 16,
                    fill: "white",
                    icon: $icon,
                }
            }
        };
    }
    let (invited_id, group_id) = match invite {
        Invite::Conversation(invite) => (invite.other_id, None),
        Invite::Group(ref invite) => (invite.invited_id, Some(invite.group_id)),
    };
    use_future(move || async move {
        CACHE
            .user_data(invited_id, credentials, &mut user_data)
            .await;
    });
    if let Some(id) = group_id {
        use_future(move || async move {
            CACHE.group_data(id, credentials, &mut group_data).await;
        });
    }
    let (group_name, group_icon) = match group_data() {
        PacketState::Response(Some(group)) => (Some(group.name), Some(icon!(GoPeople))),
        PacketState::Response(None) => {
            (Some("Deleted group".to_owned()), Some(icon!(GoCircleSlash)))
        }
        PacketState::Waiting => (
            Some("Loading group name...".to_owned()),
            Some(icon!(GoSync)),
        ),
        PacketState::ServerError(err) => (Some(format!("Error: {err}")), Some(icon!(GoAlert))),
        PacketState::RequestTimeout => (Some("Timeout".to_owned()), Some(icon!(GoAlert))),
        PacketState::NotStarted => (None, None),
    };
    let (username, email, icon) = match user_data() {
        PacketState::Response(Some(account)) => (
            account.username,
            account.email,
            match invite {
                Invite::Conversation(invite) => {
                    if invite.encrypted {
                        icon!(GoLock)
                    } else {
                        icon!(GoUnlock)
                    }
                }
                Invite::Group(_) => rsx!(),
            },
        ),
        PacketState::Response(None) => (
            Some("Deleted account".to_owned()),
            None,
            icon!(GoCircleSlash),
        ),
        PacketState::NotStarted | PacketState::Waiting => {
            (Some("Loading user data...".to_owned()), None, icon!(GoSync))
        }
        PacketState::ServerError(err) => (
            Some("Server error".to_string()),
            Some(err.to_string()),
            icon!(GoAlert),
        ),
        PacketState::RequestTimeout => {
            (Some("Request timed out".to_string()), None, icon!(GoAlert))
        }
    };
    let title = username.unwrap_or_else(|| email.clone().unwrap_or("Anonymous".to_owned()));

    rsx! {
        div {
            class: "item-panel",
            cursor: "inherit",

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

                    {title}
                    if let Ok(icon) = icon {
                        div {
                            display: "inline-block",
                            padding_left: "10px",
                            {icon}
                        }
                    }
                    if let Some(group_icon) = group_icon {
                        div {
                            display: "inline-block",
                            padding_left: "6px",
                            {group_icon}
                        }
                    }
                    if let Some(group_name) = group_name {
                        div {
                            display: "inline-block",
                            padding_left: "6px",
                            {group_name}
                        }
                    }
                }
                p {
                    padding: 0,
                    margin: 0,
                    margin_top: "6px",
                    {email}
                }
            }
            if matches!(user_data(), PacketState::Response(_)) && *cancel_result.read() == PacketState::NotStarted {
                button {
                    font_size: "16px",
                    padding: "8px 12px",
                    onclick: move |_| {
                        let invite = invite.clone();
                        async move {
                            PacketSender::default()
                                .retry_loop(|| async {
                                    match invite.clone() {
                                        Invite::Conversation(invite) => {
                                            server::cancel_dm_invite(invite.id, credentials).await
                                        }
                                        Invite::Group(invite) => {
                                            server::cancel_group_invite(invite.id, credentials).await
                                        }
                                    }
                                }, &mut cancel_result)
                                .await;
                        }
                    },
                    "Cancel"
                }
            } else {
                {status}
            }
        }
    }
}

#[component]
#[allow(non_snake_case)]
fn ReceivedInvite(invite: Invite, credentials: AccountCredentials) -> Element {
    const ICON_TRANSPARENT: Asset = asset!(
        "/assets/icon_transparent.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 97,
                height: 111,
            })
            .with_format(ImageFormat::Avif)
    );

    let mut accept_result = use_signal(|| PacketState::NotStarted);
    let mut reject_result = use_signal(|| PacketState::NotStarted);
    let mut user_data = use_signal(|| PacketState::NotStarted);
    let mut group_data = use_signal(|| PacketState::NotStarted);
    let status = match (*accept_result.read()).clone() {
        PacketState::Response(Some(group_id)) => {
            println!("Created DM group: {group_id}");
            return rsx!();
        }
        PacketState::Response(None) => {
            println!("Joined DM group");
            return rsx!();
        }
        PacketState::Waiting => rsx!(p { "Accepting..." }),
        PacketState::ServerError(err) => rsx!(p { "Server error: {err:?}" }),
        PacketState::RequestTimeout => rsx!(p { "Request timed out" }),
        PacketState::NotStarted => match (*reject_result.read()).clone() {
            PacketState::Response(()) => {
                return rsx!();
            }
            PacketState::Waiting => rsx!(p { "Rejecting..." }),
            PacketState::ServerError(err) => rsx!(p { "Server error: {err:?}" }),
            PacketState::RequestTimeout => rsx!(p { "Request timed out" }),
            PacketState::NotStarted => rsx!(),
        },
    };

    macro_rules! icon {
        ($icon:expr) => {
            rsx! {
                dioxus_free_icons::Icon {
                    width: 16,
                    height: 16,
                    fill: "white",
                    icon: $icon,
                }
            }
        };
    }
    let (inviter_id, group_id) = match invite {
        Invite::Conversation(invite) => (invite.initiator_id, None),
        Invite::Group(ref invite) => (invite.inviter_id, Some(invite.group_id)),
    };
    use_future(move || async move {
        CACHE
            .user_data(inviter_id, credentials, &mut user_data)
            .await;
    });
    if let Some(id) = group_id {
        use_future(move || async move {
            CACHE.group_data(id, credentials, &mut group_data).await;
        });
    }
    let (group_name, group_icon) = match group_data() {
        PacketState::Response(Some(group)) => (Some(group.name), Some(icon!(GoPeople))),
        PacketState::Response(None) => {
            (Some("Deleted group".to_owned()), Some(icon!(GoCircleSlash)))
        }
        PacketState::Waiting => (
            Some("Loading group name...".to_owned()),
            Some(icon!(GoSync)),
        ),
        PacketState::ServerError(err) => (Some(format!("Error: {err}")), Some(icon!(GoAlert))),
        PacketState::RequestTimeout => (Some("Timeout".to_owned()), Some(icon!(GoAlert))),
        PacketState::NotStarted => (None, None),
    };
    let (username, email, icon) = match user_data() {
        PacketState::Response(Some(account)) => (
            account.username,
            account.email,
            match invite {
                Invite::Conversation(invite) => {
                    if invite.encrypted {
                        icon!(GoLock)
                    } else {
                        icon!(GoUnlock)
                    }
                }
                Invite::Group(_) => rsx!(),
            },
        ),
        PacketState::Response(None) => (
            Some("Deleted account".to_owned()),
            None,
            icon!(GoCircleSlash),
        ),
        PacketState::NotStarted | PacketState::Waiting => {
            (Some("Loading user data...".to_owned()), None, icon!(GoSync))
        }
        PacketState::ServerError(err) => (
            Some("Server error".to_string()),
            Some(err.to_string()),
            icon!(GoAlert),
        ),
        PacketState::RequestTimeout => {
            (Some("Request timed out".to_string()), None, icon!(GoAlert))
        }
    };
    let title = username.unwrap_or_else(|| email.clone().unwrap_or("Anonymous".to_owned()));
    let invite1 = invite.clone();
    let invite2 = invite.clone();

    rsx! {
        div {
            class: "item-panel",
            cursor: "inherit",

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

                    {title}
                    if let Ok(icon) = icon {
                        div {
                            display: "inline-block",
                            padding_left: "4px",
                            {icon}
                        }
                    }
                    if let Some(group_icon) = group_icon {
                        div {
                            display: "inline-block",
                            padding_left: "6px",
                            {group_icon}
                        }
                    }
                    if let Some(group_name) = group_name {
                        div {
                            display: "inline-block",
                            padding_left: "6px",
                            {group_name}
                        }
                    }
                }
                p {
                    padding: 0,
                    margin: 0,
                    margin_top: "6px",
                    {email}
                }
            }
            if matches!(user_data(), PacketState::Response(_)) && *accept_result.read() == PacketState::NotStarted && *reject_result.read() == PacketState::NotStarted {
                button {
                    font_size: "16px",
                    padding: "8px 12px",
                    margin_right: "8px",
                    onclick: move |_| {
                        let invite = invite1.clone();
                        async move {
                            PacketSender::default()
                                .retry_loop(|| async {
                                    match invite.clone() {
                                        Invite::Conversation(invite) => {
                                            server::accept_dm_invite(invite.id, credentials).await.map(Some)
                                        }
                                        Invite::Group(invite) => {
                                            server::accept_group_invite(invite.id, credentials).await.map(|_| None)
                                        }
                                    }
                                }, &mut accept_result)
                                .await;
                        }
                    },
                    "Accept"
                }
                button {
                    font_size: "16px",
                    padding: "8px 12px",
                    onclick: move |_| {
                        let invite = invite2.clone();
                        async move {
                            PacketSender::default()
                                .retry_loop(|| async {
                                    match invite.clone() {
                                        Invite::Conversation(invite) => {
                                            server::reject_dm_invite(invite.id, credentials).await
                                        }
                                        Invite::Group(invite) => {
                                            server::reject_group_invite(invite.id, credentials).await
                                        }
                                    }
                                }, &mut reject_result)
                                .await;
                        }
                    },
                    "Reject"
                }
            } else {
                {status}
            }
        }
    }
    //
    //
    //
    // rsx! {
    //     div {
    //         p { "Invite {invite.id}" }
    //         p { "Sender: {invite.initiator_id}" }
    //         p { "Receiver: {invite.other_id}" }
    //         p { "Encrypted: {invite.encrypted}" }
    //         button {
    //             onclick: move |_| async move {
    //                 // TODO: Make that button unclickable while waiting and show an error to user
    //                 // if it occurs.
    //                 let group_id = server::accept_dm_invite(invite.id, credentials).await.unwrap();
    //                 println!("Accepted invite {} => Group {group_id} has been created", invite.id);
    //             },
    //             "Accept"
    //         }
    //         button {
    //             onclick: move |_| async move {
    //                 // TODO: Make that button unclickable while waiting and show an error to user
    //                 // if it occurs.
    //                 server::reject_dm_invite(invite.id, credentials).await.unwrap();
    //             },
    //             "Reject"
    //         }
    //     }
    // }
}
