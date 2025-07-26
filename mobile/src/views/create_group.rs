use dioxus::prelude::*;
use ::server::*;

#[component]
pub fn CreateGroup(credentials: AccountCredentials) -> Element {
    let mut group_name = use_signal(String::new);
    let mut encrypted = use_signal(|| true);
    let mut public = use_signal(|| false);
    let mut channel = use_signal(|| false);
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
            h1 { "Create a group" }
            span {
                "Name:"
                input {
                    value: "{group_name}",
                    oninput: move |event| {
                        group_name.set(event.value());
                    }
                }
            }
            "Encrypted: " input {
                r#type: "checkbox",
                checked: encrypted,
                oninput: move |_| encrypted.set(!encrypted()),
            }
            "Public: " input {
                r#type: "checkbox",
                checked: public,
                oninput: move |_| public.set(!public()),
            }
            "Channel: " input {
                r#type: "checkbox",
                checked: channel,
                oninput: move |_| channel.set(!channel()),
            }
            button {
                onclick: move |_| async move {
                    println!("Creating a group with name {group_name:?}");
                    match create_group(group_name(), None, encrypted(), public(), channel(), credentials).await {
                        Ok(group_id) => {
                            println!("Created a new group with id {group_id}");
                        }
                        Err(err) => {
                            eprintln!("Error while trying to create a new group: {err:?}");
                        }
                    };
                    let nav = navigator();
                    nav.go_back();
                },
                "Create"
            }
        }
    }
}
