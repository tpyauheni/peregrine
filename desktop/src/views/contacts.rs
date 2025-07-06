use std::{rc::Rc, time::Duration};

use chrono::Local;
use client::{cache::CACHE, future_retry_loop, packet_sender::PacketState, storage::STORAGE};
use dioxus::{logger::tracing::error, prelude::*};
use server::{
    AccountCredentials, DmGroup, DmMessage, FoundAccount, GroupMessage, MessageStatus,
    MultiUserGroup,
};
use shared::crypto;

use crate::Route;

#[component]
#[allow(non_snake_case)]
pub fn Contacts(credentials: AccountCredentials) -> Element {
    let mut found_users: Signal<Option<Vec<FoundAccount>>> = use_signal(|| None);
    let joined_dm_groups = future_retry_loop!(server::get_joined_dm_groups(credentials));
    let joined_groups = future_retry_loop!(server::get_joined_groups(credentials));
    let selected_dm_group: Signal<Option<DmGroup>> = use_signal(|| None);
    let selected_group: Signal<Option<MultiUserGroup>> = use_signal(|| None);
    let item_list = if let Some(users) = found_users() {
        if users.is_empty() {
            rsx!(h3 {
                margin: "20px",
                "No accounts are matching the search query."
            })
        } else {
            rsx! {
                for user in users {
                    User { key: user.id, account: user.clone(), credentials }
                }
            }
        }
    } else {
        match joined_dm_groups {
            PacketState::Response(dm_groups) => match joined_groups {
                PacketState::Response(groups) => {
                    if dm_groups.is_empty() && groups.is_empty() {
                        rsx!(h3 {
                            margin: "20px",
                            "You are not a member of any groups or conversations."
                        })
                    } else {
                        rsx! {
                            for group in dm_groups {
                                DmGroupPanel { key: (group.id + u64::MAX / 2), group, user_id: credentials.id, selected_dm_group, selected_group, credentials }
                            }
                            for group in groups {
                                GroupPanel { key: group.id, group: group.clone(), user_id: credentials.id, selected_dm_group, selected_group, credentials }
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
            },
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
    #[cfg(debug_assertions)]
    let debug_only_components = rsx! {
        div {
            height: "30px",
            a {
                onclick: move |_| {
                    let nav = navigator();
                    nav.push(Route::ChangeCredentials { credentials });
                },
                "Change credentials (debug-only)"
            }
        }
    };
    #[cfg(not(debug_assertions))]
    let debug_only_components = rsx!();

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
                {debug_only_components}
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
                    h2 {
                        margin: "20px",
                        "Select a group or a conversation from the menu to the left."
                    }
                }
            }
        }
    }
}

#[component]
#[allow(non_snake_case)]
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
#[allow(non_snake_case)]
fn DmMessagesPanel(selected_dm_group: DmGroup, credentials: AccountCredentials) -> Element {
    let mut msg_input: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut message: Signal<String> = use_signal(String::new);
    let sending_message: Signal<PacketState<u64>> = use_signal(|| PacketState::NotStarted);
    let mut cached_messages: Signal<Option<Vec<DmMessage>>> = use_signal(|| None);

    let mut contact_data = use_signal(|| PacketState::NotStarted);
    let contact_id = if selected_dm_group.initiator_id == credentials.id {
        selected_dm_group.other_id
    } else {
        selected_dm_group.initiator_id
    };
    use_future(move || async move {
        CACHE
            .user_data(contact_id, credentials, &mut contact_data)
            .await;
    });
    let subtitle = match contact_data() {
        PacketState::Response(data) => {
            data.map_or(format!("[Deleted account {contact_id}]"), |data| {
                data.username.unwrap_or(
                    data.email
                        .unwrap_or(format!("[Anonymous user {contact_id}]")),
                )
            })
        }
        _ => format!("[Account {contact_id}]"),
    };
    // TODO: Store the title in `Storage` and then load it.
    // let title = format!("[Group {}]", group.id);
    let title = subtitle.clone();

    future_retry_loop! { dm_messages_signal, dm_messages_resource, server::fetch_new_dm_messages(selected_dm_group.id, 0, credentials) };
    use_effect(move || {
        if let PacketState::Response(mut messages) = dm_messages_signal() {
            messages.reverse();
            cached_messages.set(Some(messages.clone()));
        }
    });
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            dm_messages_resource.restart();
        }
    });

    // TODO: Store `last_received_message_id` and received messages in `Storage`.
    let messages = if let Some(messages) = cached_messages() {
        rsx! {
            for message in messages {
                DmMessageComponent { contact_id, message }
            }
        }
    } else {
        match dm_messages_signal() {
            PacketState::Response(mut messages) => {
                messages.reverse();
                rsx! {
                    for message in messages {
                        DmMessageComponent { contact_id, message }
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
        }
    };
    let sending_messages = match sending_message() {
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
                class: "imitate-button",
                width: "100%",
                max_width: "calc(100% - 32px)",
                height: "56px",
                min_height: "56px",
                padding: "16px",
                onclick: move |_| async move {
                    let nav = navigator();
                    nav.push(Route::OtherUserAccount { user_id: contact_id, credentials });
                },

                h1 {
                    margin_top: "10px",
                    margin_bottom: 0,
                    margin_left: "16px",

                    {title}
                }
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
                padding: "16px",
                background_color: "#121519",
                onclick: move |_| async move {
                    let Some(msg_input) = msg_input() else {
                        return;
                    };
                    _ = msg_input.set_focus(true).await;
                },

                textarea {
                    id: "main-msg-input",
                    class: "imitate-input msg-textbox no-scrollbar",
                    role: "textbox",
                    value: "{message}",
                    onmounted: move |cx| msg_input.set(Some(cx.data())),
                    oninput: move |event| async move {
                        message.set(event.value());
                        document::eval(r#"let input = document.getElementById("main-msg-input");
                            let height = input.scrollHeight;
                            if (height > 300) {
                                input.style = "height: 300px";
                            } else {
                                input.style = "height: " + height + "px";
                            }"#).await.unwrap();
                        // if let Some(msg_input) = msg_input() {
                            // let scroll_size = msg_input.get_scroll_size().await.unwrap_or(Size2D::zero());
                            // msg_input.set_style(format!("height: {}px", scroll_size.height));
                            // msg_input;
                            //scroll_size.height
                        // }
                    },
                    onkeydown: move |event| async move {
                        if event.code() != Code::Enter || event.modifiers().shift() {
                            return;
                        }
                        event.prevent_default();
                        let content = message();
                        let (msg_bytes, encryption_method): (Box<[u8]>, String) = if let Some((algorithm_name, key)) = STORAGE.load_dm_key(selected_dm_group.id) {
                            (
                                crypto::symmetric_encrypt(&algorithm_name, content.as_bytes(), &key).unwrap(),
                                crypto::to_encryption_method(&algorithm_name),
                            )
                        } else {
                            (Box::from(content.clone().as_bytes()), "plain".to_owned())
                        };
                        println!("Send result: {:?}", server::send_dm_message(
                            selected_dm_group.id,
                            encryption_method,
                            msg_bytes,
                            credentials,
                        ).await);
                        // PacketSender::default()
                        //     .retry_loop(move || server::send_dm_message(
                        //         selected_dm_group.id,
                        //         "plain".to_owned(),
                        //         msg_bytes.clone(),
                        //         credentials,
                        //     ), &mut sending_message).await;
                        // println!("Sending message: {content:?}");
                        message.set(String::new());
                        dm_messages_resource.restart();
                        document::eval(r#"let input = document.getElementById("main-msg-input");
                            input.style = "height: 36px";"#).await.unwrap();
                    }
                }
            }
        }
    }
}

#[component]
#[allow(non_snake_case)]
fn GroupMessagesPanel(selected_group: MultiUserGroup, credentials: AccountCredentials) -> Element {
    let mut msg_input: Signal<Option<Rc<MountedData>>> = use_signal(|| None);
    let mut message: Signal<String> = use_signal(String::new);
    let sending_message: Signal<PacketState<u64>> = use_signal(|| PacketState::NotStarted);
    let mut cached_messages: Signal<Option<Vec<GroupMessage>>> = use_signal(|| None);

    future_retry_loop! { group_messages_signal, group_messages_resource, server::fetch_new_group_messages(selected_group.id, 0, credentials) };
    use_effect(move || {
        if let PacketState::Response(mut messages) = group_messages_signal() {
            messages.reverse();
            cached_messages.set(Some(messages));
        }
    });
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_secs(5)).await;
            group_messages_resource.restart();
        }
    });

    // TODO: Store `last_received_message_id` and received messages in `Storage`.
    let messages = if let Some(messages) = cached_messages() {
        rsx! {
            for message in messages {
                GroupMessageComponent { message, self_id: credentials.id, credentials, group_id: selected_group.id }
            }
        }
    } else {
        match group_messages_signal() {
            PacketState::Response(mut messages) => {
                messages.reverse();
                rsx! {
                    for message in messages {
                        GroupMessageComponent { message, self_id: credentials.id, credentials, group_id: selected_group.id }
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
        }
    };
    let sending_messages = match sending_message() {
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
                class: "imitate-button",
                width: "100%",
                max_width: "calc(100% - 32px)",
                height: "56px",
                min_height: "56px",
                padding: "16px",
                onclick: move |_| async move {
                    let nav = navigator();
                    nav.push(Route::GroupMenu { group_id: selected_group.id, credentials });
                },

                h1 {
                    margin_top: "10px",
                    margin_bottom: 0,
                    margin_left: "16px",

                    {selected_group.name}
                }
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
                    let Some(msg_input) = msg_input() else {
                        return;
                    };
                    _ = msg_input.set_focus(true).await;
                },

                textarea {
                    id: "main-msg-input",
                    class: "imitate-input msg-textbox no-scrollbar",
                    role: "textbox",
                    value: "{message}",
                    onmounted: move |cx| msg_input.set(Some(cx.data())),
                    oninput: move |event| async move {
                        message.set(event.value());
                        document::eval(r#"let input = document.getElementById("main-msg-input");
                            let height = input.scrollHeight;
                            if (height > 300) {
                                input.style = "height: 300px";
                            } else {
                                input.style = "height: " + height + "px";
                            }"#).await.unwrap();
                    },
                    onkeydown: move |event| async move {
                        if event.code() != Code::Enter || event.modifiers().shift() {
                            return;
                        }
                        event.prevent_default();
                        let content = message();
                        let (msg_bytes, encryption_method): (Box<[u8]>, String) = if let Some((algorithm_name, key)) = STORAGE.load_group_key(selected_group.id) {
                            (
                                crypto::symmetric_encrypt(&algorithm_name, content.as_bytes(), &key).unwrap(),
                                crypto::to_encryption_method(&algorithm_name),
                            )
                        } else {
                            (Box::from(content.clone().as_bytes()), "plain".to_owned())
                        };
                        println!("Send result: {:?}", server::send_group_message(
                            selected_group.id,
                            encryption_method,
                            msg_bytes,
                            credentials,
                        ).await);
                        println!("Sending group message: {content:?}");
                        message.set(String::new());
                        group_messages_resource.restart();
                        document::eval(r#"let input = document.getElementById("main-msg-input");
                            input.style = "height: 36px";"#).await.unwrap();
                    }
                }
            }
        }
    }
}

#[component]
#[allow(non_snake_case)]
pub fn DmGroupPanel(
    group: DmGroup,
    user_id: u64,
    selected_dm_group: Signal<Option<DmGroup>>,
    selected_group: Signal<Option<MultiUserGroup>>,
    credentials: AccountCredentials,
) -> Element {
    const ICON_TRANSPARENT: Asset = asset!(
        "/assets/icon_transparent.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 97,
                height: 111,
            })
            .with_format(ImageFormat::Avif)
    );

    let mut contact_data = use_signal(|| PacketState::NotStarted);
    let contact_id = if group.initiator_id == user_id {
        group.other_id
    } else {
        group.initiator_id
    };
    use_future(move || async move {
        CACHE
            .user_data(contact_id, credentials, &mut contact_data)
            .await;
    });
    let subtitle = match contact_data() {
        PacketState::Response(data) => {
            data.map_or(format!("[Deleted account {contact_id}]"), |data| {
                data.username.unwrap_or(
                    data.email
                        .unwrap_or(format!("[Anonymous user {contact_id}]")),
                )
            })
        }
        _ => format!("[Account {contact_id}]"),
    };
    // TODO: Store the title in `Storage` and then load it.
    // let title = format!("[Group {}]", group.id);
    let title = subtitle.clone();
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
#[allow(non_snake_case)]
fn DmMessageComponent(contact_id: u64, message: DmMessage) -> Element {
    const ICON_MSG_STATUS_SENT: Asset = asset!(
        "/assets/msg_status_sent_icon.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 16,
                height: 16,
            })
            .with_format(ImageFormat::Avif)
    );
    const ICON_MSG_STATUS_DELIVERED: Asset = asset!(
        "/assets/msg_status_delivered_icon.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 19,
                height: 16,
            })
            .with_format(ImageFormat::Avif)
    );
    let message_content = if message.encryption_method != "plain" {
        if let Some(key) = STORAGE.load_dm_key(contact_id) {
            if let Some(Some(plaintext)) =
                crypto::symmetric_decrypt(&key.0, message.content, &key.1)
            {
                let plain_string = String::from_utf8_lossy(&plaintext);
                rsx!({ plain_string })
            } else {
                rsx!(p { style: "color:#f00", "Failed to decrypt message" })
            }
        } else {
            rsx!(p { style: "color:#f00", "Failed to decrypt message" })
        }
    } else {
        let plain_string = String::from_utf8_lossy(&message.content);
        rsx!({ plain_string })
    };
    let sent_by_me = message.status != MessageStatus::SentByOther;
    let time = if let Some(time) = message.sent_time {
        let utc = time.and_local_timezone(Local).unwrap();
        utc.time().format("%H:%M").to_string()
    } else {
        "??:??".to_owned()
    };
    rsx! {
        div {
            class: {format!("message {}", if sent_by_me {
                "msg-me"
            } else {
                "msg-other"
            })},

            {message_content}
            div {
                class: "msg-info",

                if sent_by_me {
                    p {
                        class: "time-text time-text-me",
                        {time}
                    }
                    if message.status == MessageStatus::Sent {
                        img {
                            src: ICON_MSG_STATUS_SENT,
                            alt: "Sent",
                            class: "msg-status-icon msg-status-sent",
                        }
                    } else if message.status == MessageStatus::Delivered {
                        img {
                            src: ICON_MSG_STATUS_DELIVERED,
                            alt: "Delivered",
                            class: "msg-status-icon msg-status-delivered",
                        }
                    }
                } else {
                    p {
                        class: "time-text time-text-other",
                        {time}
                    }
                }
            }
        }
        br {}
    }
}

#[component]
#[allow(non_snake_case)]
pub fn GroupPanel(
    group: MultiUserGroup,
    user_id: u64,
    selected_dm_group: Signal<Option<DmGroup>>,
    selected_group: Signal<Option<MultiUserGroup>>,
    credentials: AccountCredentials,
) -> Element {
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
    let members_data = future_retry_loop!(server::get_group_member_count(group.id, credentials));
    let subtitle = match members_data {
        PacketState::Response(members) => {
            if members == 1 {
                "1 member".to_owned()
            } else {
                format!("{members} members")
            }
        }
        _ => format!("[Group {}]", group.id),
    };
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
#[allow(non_snake_case)]
fn GroupMessageComponent(
    message: GroupMessage,
    self_id: u64,
    credentials: AccountCredentials,
    group_id: u64,
) -> Element {
    let mut author_data = use_signal(|| PacketState::NotStarted);
    let author_id = message.sender_id;
    use_future(move || async move {
        CACHE
            .user_data(author_id, credentials, &mut author_data)
            .await;
    });
    let author = match author_data() {
        PacketState::Response(data) => {
            rsx! {
                h3 {
                    margin_top: "12px",
                    margin_bottom: "4px",
                    {data.map_or(format!("[Deleted account {author_id}]"), |data| data.username.unwrap_or(data.email.unwrap_or(format!("[Anonymous user {author_id}]"))))}
                }
            }
        }
        _ => rsx! {
            h3 {
                margin_top: "12px",
                margin_bottom: "4px",
                "[Account {author_id}]"
            }
        },
    };
    let sent_by_me = message.sender_id == self_id;
    let time = if let Some(time) = message.sent_time {
        let utc = time.and_local_timezone(Local).unwrap();
        utc.time().format("%H:%M").to_string()
    } else {
        "??:??".to_owned()
    };
    let message_content = if message.encryption_method != "plain" {
        if let Some(key) = STORAGE.load_group_key(group_id) {
            if let Some(Some(plaintext)) =
                crypto::symmetric_decrypt(&key.0, message.content, &key.1)
            {
                rsx!({ String::from_utf8_lossy(&plaintext) })
            } else {
                rsx!(p { style: "color:#f00", "Failed to decrypt message" })
            }
        } else {
            rsx!(p { style: "color:#f00", "Failed to decrypt message" })
        }
    } else {
        rsx!({ String::from_utf8_lossy(&message.content) })
    };
    rsx! {
        {author}
        div {
            class: {format!("message {}", if sent_by_me {
                "msg-me"
            } else {
                "msg-other"
            })},

            {message_content}
            div {
                class: "msg-info",

                if sent_by_me {
                    p {
                        class: "time-text time-text-me",
                        {time}
                    }
                } else {
                    p {
                        class: "time-text time-text-other",
                        {time}
                    }
                }
            }
        }
        br {}
    }
}
