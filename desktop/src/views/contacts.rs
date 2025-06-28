use std::{rc::Rc, time::Duration};

use client::{future_retry_loop, packet_sender::{PacketSender, PacketState}};
use dioxus::{logger::tracing::error, prelude::*};
use server::{AccountCredentials, DmGroup, DmMessage, FoundAccount, GroupMessage, MultiUserGroup};

use crate::Route;

#[component]
pub fn Contacts(credentials: AccountCredentials) -> Element {
    let mut found_users: Signal<Option<Vec<FoundAccount>>> = use_signal(|| None);
    let joined_dm_groups = future_retry_loop!(server::get_joined_dm_groups(credentials));
    let joined_groups = future_retry_loop!(server::get_joined_groups(credentials));
    let selected_dm_group: Signal<Option<DmGroup>> = use_signal(|| None);
    let selected_group: Signal<Option<MultiUserGroup>> = use_signal(|| None);
    let item_list = if let Some(users) = found_users() {
        if users.is_empty() {
            rsx!(h3 { "No accounts are matching the search query" })
        } else {
            rsx! {
                for user in users {
                    User { key: user.id, account: user.clone(), credentials }
                }
            }
        }
    } else {
        match joined_dm_groups {
            PacketState::Response(dm_groups) => {
                match joined_groups {
                    PacketState::Response(groups) => {
                        if dm_groups.is_empty() && groups.is_empty() {
                            rsx!(h3 { "You are not a member of any groups or conversations" })
                        } else {
                            rsx! {
                                for group in dm_groups {
                                    DmGroupPanel { key: (group.id + u64::MAX / 2), group: group.clone(), user_id: credentials.id, selected_dm_group, selected_group }
                                }
                                for group in groups {
                                    GroupPanel { key: group.id, group: group.clone(), user_id: credentials.id, selected_dm_group, selected_group }
                                }
                            }
                        }
                    }
                    PacketState::Waiting => {
                        rsx!(h3 { "Loading..." })
                    }
                    PacketState::ServerError(err) => {
                        rsx!(h3 { "Server error: {err:?}" })
                    }
                    PacketState::RequestTimeout => {
                        rsx!(h3 { "Request timeout" })
                    }
                    PacketState::NotStarted => unreachable!(),
                }
            }
            PacketState::Waiting => {
                rsx!(h3 { "Loading..." })
            }
            PacketState::ServerError(err) => {
                rsx!(h3 { "Server error: {err:?}" })
            }
            PacketState::RequestTimeout => {
                rsx!(h3 { "Request timeout" })
            }
            PacketState::NotStarted => unreachable!(),
        }
    };

    rsx! {
        div {
            class: "twopanel-container",

            div {
                class: "twopanel twopanel-left",
                min_width: "400px",
                max_width: "30vw",
                display: "flex",
                flex_direction: "column",
                height: "100%",
                input {
                    width: "100%",
                    height: "32px",
                    border: "none",
                    background_color: "#202427",
                    placeholder: "Search",
                    oninput: move |event| async move {
                        let query = event.value();

                        if query.is_empty() {
                            found_users.set(None);
                        } else {
                            match server::find_user(query, credentials).await {
                                Ok(data) => found_users.set(Some(data)),
                                Err(err) => error!("Error while trying to find user: {err:?}"),
                            };
                        }
                    }
                }
                div {
                    margin_top: "8px",
                    class: "noselect",

                    {item_list}
                }
                div {
                    flex_grow: 1,
                    margin: 0,
                }
                div {
                    height: "30px",
                    a {
                        onclick: move |_| {
                            let nav = navigator();
                            nav.push(Route::Invites { credentials });
                        },
                        "Invites",
                    }
                }
                div {
                    height: "30px",
                    a {
                        onclick: move |_| {
                            let nav = navigator();
                            nav.push(Route::CreateGroup { credentials });
                        },
                        "Create a new group",
                    }
                }
            }
            div {
                class: "twopanel twopanel-right",
                if let Some(dm_group) = selected_dm_group() {
                    DmMessagesPanel { selected_dm_group: dm_group, credentials }
                } else if let Some(group) = selected_group() {
                    GroupMessagesPanel { selected_group: group, credentials }
                } else {
                    h2 { "Select a group or a conversation from the menu to the left" }
                }
            }
        }
    }
}

#[component]
pub fn User(account: FoundAccount, credentials: AccountCredentials) -> Element {
    const ICON_TRANSPARENT: Asset = asset!(
        "/assets/icon_transparent.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 97,
                height: 111,
            })
            .with_format(ImageFormat::Avif)
    );

    let title = account
        .username
        .unwrap_or(account.email.clone().unwrap_or("Anonymous".to_owned()));
    let email = account.email.unwrap_or("Hidden email".to_owned());
    rsx! {
        div {
            class: "item-panel",
            onclick: move |_| async move {
                let nav = navigator();
                nav.push(Route::OtherUserAccount { user_id: account.id, credentials });
            },

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
fn DmMessagesPanel(selected_dm_group: DmGroup, credentials: AccountCredentials) -> Element {
    let mut msg_input: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut message: Signal<String> = use_signal(String::new);
    let mut sending_message: Signal<PacketState<u64>> = use_signal(|| PacketState::NotStarted);

    future_retry_loop! { dm_messages_signal, dm_messages_resource, server::fetch_new_dm_messages(selected_dm_group.id, 0, credentials) };
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            dm_messages_resource.restart();
        }
    });

    // TODO: Store `last_received_message_id` and received messages in `Storage`.
    let messages = match (dm_messages_signal()).clone() {
        PacketState::Response(mut messages) => {
            messages.reverse();
            rsx! {
                for message in messages {
                    DmMessageComponent { message }
                    // h4 { {format!("Message {} with encryption method \"{}\": {:?}", message.id, message.encryption_method, message.content)} }
                }
            }
        }
        PacketState::Waiting => {
            rsx!(h1 { "Loading messages..." })
        }
        PacketState::ServerError(err) => {
            rsx!(h1 { "Server error: {err}" })
        }
        PacketState::RequestTimeout => {
            rsx!(h1 { "Request timeout" })
        }
        PacketState::NotStarted => unreachable!(),
    };
    let sending_messages = match (*sending_message.read()).clone() {
        PacketState::Response(_) | PacketState::NotStarted => {
            rsx!()
        }
        PacketState::Waiting => {
            rsx!(h4 { "Sending message..." })
        }
        PacketState::ServerError(err) => {
            rsx!(h4 { "Error while trying to send a message: {err}" })
        }
        PacketState::RequestTimeout => {
            rsx!(h4 { "Request timed out" })
        }
    };

    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            height: "100%",
            max_height: "100vh",

            div {
                width: "100%",
                max_width: "calc(100% - 32px)",
                height: "80px",
                padding: "16px",

                h1 { {format!("Group {}", selected_dm_group.id)} }
            }
            div {
                width: "100%",
                height: "1px",
                background_image: "linear-gradient(#2b2b2b00, #2b2b2bff, #2b2b2b00)",

                br {}
            }
            div {
                width: "100%",
                max_width: "calc(100% - 32px)",
                flex_grow: 1,
                overflow: "auto",
                padding: "16px",

                // h3 { "Messages here:" }
                // for i in 0..100 {
                //     h4 { {format!("Message {i}!")} }
                // }
                {messages}
                {sending_messages}
            }
            div {
                width: "100%",
                height: "1px",
                background_image: "linear-gradient(#2b2b2b00, #2b2b2bff, #2b2b2b00)",

                br {}
            }
            div {
                width: "100%",
                max_width: "calc(100% - 32px)",
                height: "auto",
                // height: "34px",
                padding: "16px",
                background_color: "#121519",
                onclick: move |_| async move {
                    let Some(msg_input) = msg_input.read().clone() else {
                        return;
                    };
                    _ = msg_input.set_focus(true).await;
                },

                // TODO: Unset `contenteditable` and make text input work from keyboard using
                // events.
                textarea {
                    class: "imitate-input",
                    role: "textbox",
                    contenteditable: true,
                    resize: "none",
                    value: "{message}",
                    border: "none",
                    background: "none",
                    padding: 0,
                    height: "auto",
                    onmounted: move |cx| msg_input.set(Some(cx.data())),
                    oninput: move |event| {
                        message.set(event.value());
                    },
                    onkeydown: move |event| async move {
                        if event.code() != Code::Enter || event.modifiers().shift() {
                            return;
                        }
                        event.prevent_default();
                        // TODO: Encryption.
                        let content = (*message.read()).clone();
                        let msg_bytes: Box<[u8]> = Box::from(content.clone().as_bytes());
                        println!("Send result: {:?}", server::send_dm_message(
                            selected_dm_group.id,
                            "plain".to_owned(),
                            msg_bytes.clone(),
                            credentials,
                        ).await);
                        // PacketSender::default()
                        //     .retry_loop(move || server::send_dm_message(
                        //         selected_dm_group.id,
                        //         "plain".to_owned(),
                        //         msg_bytes.clone(),
                        //         credentials,
                        //     ), &mut sending_message).await;
                        println!("Sending message: {content:?}");
                        message.set(String::new());
                        dm_messages_resource.restart();
                    }
                }
            }
        }
    }
}

#[component]
fn GroupMessagesPanel(selected_group: MultiUserGroup, credentials: AccountCredentials) -> Element {
    let mut msg_input: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut message: Signal<String> = use_signal(String::new);
    let mut sending_message: Signal<PacketState<u64>> = use_signal(|| PacketState::NotStarted);

    future_retry_loop! { group_messages_signal, group_messages_resource, server::fetch_new_group_messages(selected_group.id, 0, credentials) };
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            group_messages_resource.restart();
        }
    });

    // TODO: Store `last_received_message_id` and received messages in `Storage`.
    let messages = match (*group_messages_signal.read()).clone() {
        PacketState::Response(mut messages) => {
            messages.reverse();
            rsx! {
                for message in messages {
                    GroupMessageComponent { message, self_id: credentials.id }
                    // h4 { {format!("Message {} with encryption method \"{}\": {:?}", message.id, message.encryption_method, message.content)} }
                }
            }
        }
        PacketState::Waiting => {
            rsx!(h1 { "Loading messages..." })
        }
        PacketState::ServerError(err) => {
            rsx!(h1 { "Server error: {err}" })
        }
        PacketState::RequestTimeout => {
            rsx!(h1 { "Request timeout" })
        }
        PacketState::NotStarted => unreachable!(),
    };
    let sending_messages = match (*sending_message.read()).clone() {
        PacketState::Response(_) | PacketState::NotStarted => {
            rsx!()
        }
        PacketState::Waiting => {
            rsx!(h4 { "Sending message..." })
        }
        PacketState::ServerError(err) => {
            rsx!(h4 { "Error while trying to send a message: {err}" })
        }
        PacketState::RequestTimeout => {
            rsx!(h4 { "Request timed out" })
        }
    };

    rsx! {
        div {
            display: "flex",
            flex_direction: "column",
            height: "100%",
            max_height: "100vh",

            div {
                width: "100%",
                max_width: "calc(100% - 32px)",
                height: "80px",
                padding: "16px",

                h1 { {format!("Group {:?}", selected_group.name)} }
            }
            div {
                width: "100%",
                height: "1px",
                background_image: "linear-gradient(#2b2b2b00, #2b2b2bff, #2b2b2b00)",

                br {}
            }
            div {
                width: "100%",
                max_width: "calc(100% - 32px)",
                flex_grow: 1,
                overflow: "auto",
                padding: "16px",

                // h3 { "Messages here:" }
                // for i in 0..100 {
                //     h4 { {format!("Message {i}!")} }
                // }
                {messages}
                {sending_messages}
            }
            div {
                width: "100%",
                height: "1px",
                background_image: "linear-gradient(#2b2b2b00, #2b2b2bff, #2b2b2b00)",

                br {}
            }
            div {
                width: "100%",
                max_width: "calc(100% - 32px)",
                height: "auto",
                // height: "34px",
                padding: "16px",
                background_color: "#121519",
                onclick: move |_| async move {
                    let Some(msg_input) = msg_input.read().clone() else {
                        return;
                    };
                    _ = msg_input.set_focus(true).await;
                },

                // TODO: Unset `contenteditable` and make text input work from keyboard using
                // events.
                textarea {
                    class: "imitate-input",
                    role: "textbox",
                    contenteditable: true,
                    resize: "none",
                    value: "{message}",
                    border: "none",
                    background: "none",
                    padding: 0,
                    height: "auto",
                    onmounted: move |cx| msg_input.set(Some(cx.data())),
                    oninput: move |event| {
                        message.set(event.value());
                    },
                    onkeydown: move |event| async move {
                        if event.code() != Code::Enter || event.modifiers().shift() {
                            return;
                        }
                        event.prevent_default();
                        // TODO: Encryption.
                        let content = (*message.read()).clone();
                        let msg_bytes: Box<[u8]> = Box::from(content.clone().as_bytes());
                        println!("Send result: {:?}", server::send_group_message(
                            selected_group.id,
                            "plain".to_owned(),
                            msg_bytes.clone(),
                            credentials,
                        ).await);
                        println!("Sending group message: {content:?}");
                        message.set(String::new());
                        group_messages_resource.restart();
                    }
                }
            }
        }
    }
}

#[component]
pub fn DmGroupPanel(group: DmGroup, user_id: u64, selected_dm_group: Signal<Option<DmGroup>>, selected_group: Signal<Option<MultiUserGroup>>) -> Element {
    const ICON_TRANSPARENT: Asset = asset!(
        "/assets/icon_transparent.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 97,
                height: 111,
            })
            .with_format(ImageFormat::Avif)
    );

    // TODO: Store the title in `Storage` and then load it.
    let title = group.id.to_string();
    // TODO: Make `identify_user(id: u64)` function which will check for client-overriden (or at
    // least cached) data in `Storage` and if it doesn't find it, it'll send a request to the
    // server and store the result.
    let subtitle = if group.initiator_id == user_id {
        group.other_id
    } else {
        group.initiator_id
    }.to_string();
    rsx! {
        div {
            class: "item-panel",
            onclick: move |_| async move {
                selected_dm_group.set(Some(group));
                selected_group.set(None);
            },

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
                }
                p {
                    padding: 0,
                    margin: 0,
                    margin_top: "6px",
                    {subtitle}
                }
            }
        }
    }
}

#[component]
fn DmMessageComponent(message: DmMessage) -> Element {
    rsx! {
        div {
            class: {format!("message {}", if message.sent_by_me {
                "msg-me"
            } else {
                "msg-other"
            })},

            {String::from_utf8_lossy(&message.content)}
        }
        br {}
    }
}

#[component]
pub fn GroupPanel(group: MultiUserGroup, user_id: u64, selected_dm_group: Signal<Option<DmGroup>>, selected_group: Signal<Option<MultiUserGroup>>) -> Element {
    const ICON_TRANSPARENT: Asset = asset!(
        "/assets/icon_transparent.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 97,
                height: 111,
            })
            .with_format(ImageFormat::Avif)
    );

    // TODO: Store the title in `Storage` and then load it (if overriden).
    let title = group.name.clone();
    let subtitle = group.id.to_string();
    rsx! {
        div {
            class: "item-panel",
            onclick: move |_| {
                let group_clone = group.clone();
                async move {
                    selected_group.set(Some(group_clone));
                    selected_dm_group.set(None);
                }
            },

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
                }
                p {
                    padding: 0,
                    margin: 0,
                    margin_top: "6px",
                    {subtitle}
                }
            }
        }
    }
}

#[component]
fn GroupMessageComponent(message: GroupMessage, self_id: u64) -> Element {
    rsx! {
        div {
            class: {format!("message {}", if message.sender_id == self_id {
                "msg-me"
            } else {
                "msg-other"
            })},

            {String::from_utf8_lossy(&message.content)}
        }
        br {}
    }
}
