use dioxus::{logger::tracing::error, prelude::*};
use server::{AccountCredentials, FoundAccount};

use crate::Route;

#[component]
pub fn Contacts(credentials: AccountCredentials) -> Element {
    let mut found_users: Signal<Vec<FoundAccount>> = use_signal(Vec::new);
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
                        match server::find_user(query, credentials).await {
                            Ok(data) => found_users.set(data),
                            Err(err) => error!("Error while trying to find user: {err:?}"),
                        };
                    }
                }
                div {
                    margin_top: "8px",
                    class: "noselect",

                    for user in found_users() {
                        User { key: user.id, account: user.clone(), credentials }
                    }
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
            }
            div {
                class: "twopanel twopanel-right",
                h1 { "Panel 2" }
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
                match server::send_dm_invite(account.id, false, credentials).await {
                    Ok(invite_id) => {
                        println!("Sent invite: {invite_id:?}");
                    }
                    Err(err) => {
                        eprintln!("Error from server: {err:?}");
                    }
                }
                println!("User {:?} clicked", account.id);
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
