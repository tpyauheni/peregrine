use dioxus::{logger::tracing::error, prelude::*};
use server::{AccountCredentials, FoundAccount};

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
                        User { key: user.id, account: user.clone() }
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
pub fn User(account: FoundAccount) -> Element {
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
            class: "panel-nonround",
            width: "100%",
            height: "48px",
            padding: "16px",
            padding_top: "12px",
            padding_bottom: "12px",
            display: "flex",
            align_items: "center",
            justify_content: "center",

            div {
                flex: "0 3 48px",

                img {
                    src: ICON_TRANSPARENT,
                    margin_right: "24px",
                    width: "48px",
                    height: "48px",
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
                    {email}
                }
            }
        }
    }
}
