use client::storage::STORAGE;
use dioxus::{logger::tracing::{error, info}, prelude::*};
use server::{AccountCredentials, SessionParams};
use shared::{crypto::{self, AsymmetricCipherPrivate, AsymmetricCipherPublic}, limits::LIMITS};

use crate::Route;

const DEFAULT_SERVER: &str = "peregrine.werryxgames.com";

fn check_email(email: &str) -> Option<String> {
    // TODO: Use some crate for email-checking.
    // It is way harder than I expected.

    if email.is_empty() {
        Some("Email is a required field".to_owned())
    } else if email.len() < 3 {
        Some("Email is too short".to_owned())
    } else if !email.contains('@') {
        Some("Email must contain \"@\" symbol".to_owned())
    } else if !email.is_ascii() {
        Some("Email must be specified in ASCII encoding".to_owned())
    } else if email.chars().any(|x| x.is_ascii_control()) {
        Some("Email can't contain ASCII control characters".to_owned())
    } else {
        let index = email.find('@').unwrap();

        if index == 0 {
            return Some("\"@\" symbol can't be the first in an email address".to_owned());
        }
        if index == email.len() - 1 {
            return Some("\"@\" symbol can't be the last in an email address".to_owned());
        }

        if index != email.rfind('@').unwrap() {
            return Some("Quoted characters in emails are not yet supported".to_owned());
        }

        for chr in "()<>,;:\\\"[]".chars() {
            if email.contains(chr) {
                return Some("Quoted characters in emails are not yet supported".to_owned());
            }
        }

        let (name, host) = email.split_once('@').unwrap();

        for part in [name, host] {
            if part.is_empty() {
                return Some("Email can't contain any empty parts".to_owned());
            }

            let mut iter = part.bytes();

            if iter.next() == Some('.'.try_into().unwrap()) {
                return Some("Parts in email can't start with a dot (\".\")".to_owned());
            }
            if part.bytes().last() == Some('.'.try_into().unwrap()) {
                return Some("Parts in email can't end with a dot (\".\")".to_owned());
            }

            let mut prev_dot: bool = false;

            for chr in iter {
                if chr == <char as TryInto<u8>>::try_into('.').unwrap() {
                    if prev_dot {
                        return Some(
                            "Quoted characters in emails are not yet supported".to_owned(),
                        );
                    }
                    prev_dot = true;
                } else {
                    prev_dot = false;
                }
            }
        }

        None
    }
}

fn check_username(_username: &str) -> Option<String> {
    None
}

fn check_password(password: &str) -> Option<String> {
    // TODO: Use some crate for password security checking

    if password.len() >= 32 {
        // Even if user is using weak password, it won't be bruteforceable at 32+ length.
        // I'm just using password manager and I hate when I'm pasting very long password
        // which contains large amounts of different obscure characters but not a single digit
        // so it's not letting me create an account.
        None
    } else if password.len() < 8 {
        Some("Password must be at least 8 characters long".to_owned())
    } else if !password.chars().any(|x| x.is_ascii_digit()) {
        Some("Password must contain at least one digit".to_owned())
    } else if !password.chars().any(|x| x.is_ascii_alphabetic()) {
        Some("Password must contain at least one letter".to_owned())
    } else {
        None
    }
}

fn check_server(server: &str) -> Option<String> {
    // TODO: Use some crate for hostname/IP checking

    if server == DEFAULT_SERVER {
        return None;
    }

    None
}


#[component]
pub fn RegisterAccount() -> Element {
    const PANEL_WIDTH: u32 = 480;
    const PANEL_MARGIN_WIDTH: u32 = 48;
    const PANEL_MARGIN_HEIGHT: u32 = 36;
    const INNER_PANEL_WIDTH: u32 = PANEL_WIDTH - PANEL_MARGIN_WIDTH * 2;
    const ICON_TRANSPARENT: Asset = asset!(
        "/assets/icon_transparent.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 97,
                height: 111,
            })
            .with_format(ImageFormat::Avif)
    );

    let error: Signal<Option<String>> = use_signal(|| None);
    let mut advanced_mode: Signal<bool> = use_signal(|| false);
    let mut last_entered_server: Signal<String> = use_signal(|| "".to_owned());

    async fn create_account(event: Event<FormData>, mut error_sig: Signal<Option<String>>) -> () {
        let values = event.values();
        let email: &str = &values["email"].as_value();
        let username: &str = &values["username"].as_value();
        let password: &str = &values["password"].as_value();
        let server: String = if values.contains_key("server") {
            let value = values["server"].as_value();
            if value.is_empty() {
                DEFAULT_SERVER.to_owned()
            } else {
                value
            }
        } else {
            DEFAULT_SERVER.to_owned()
        };

        if let Some(error) = check_email(email) {
            info!("Invalid user input: email verification error: '{}'", error);
            error_sig.set(Some(error.clone()));
            return;
        }
        if let Some(error) = check_username(username) {
            error_sig.set(Some(error.clone()));
            return;
        }
        if let Some(error) = check_password(password) {
            error_sig.set(Some(error.clone()));
            return;
        }
        if let Some(error) = check_server(&server) {
            error_sig.set(Some(error.clone()));
            return;
        }

        let cryptoset = shared::crypto::default_cryptoset(password.as_bytes(), None);
        let public_key = cryptoset.asymmetric_cipher.into_public_key_bytes();
        info!(
            "Submitting form: email='{email}', username='{username}', server='{server}', public_key={public_key:?}"
        );
        error_sig.set(None);
        let (_, x3dh_public) = STORAGE.x3dh_data(crypto::preferred_alogirthm());
        let (account_id, session_token) = server::create_account(
            email.to_owned(),
            username.to_owned(),
            public_key,
            x3dh_public,
        )
        .await
        .unwrap();
        let login_credentials = AccountCredentials {
            id: account_id,
            session_token,
        };
        STORAGE.store_session_credentials(login_credentials);
        let nav = navigator();
        nav.replace(Route::Contacts {
            credentials: login_credentials,
        });
        info!("Form submitted, session token: {session_token:?}");
    }

    rsx! {
        div {
            id: "center-container",

            div {
                id: "main-panel",
                class: "panel noselect",
                width: format!("{PANEL_WIDTH}px"),
                max_height: "94vh",

                div {
                    id: "inside-container",
                    margin_left: format!("{PANEL_MARGIN_WIDTH}px"),
                    width: format!("{INNER_PANEL_WIDTH}px"),
                    margin_top: format!("{PANEL_MARGIN_HEIGHT}px"),

                    div {
                        display: "flex",
                        margin_left: "46px",
                        margin_right: "32px",
                        img {
                            src: ICON_TRANSPARENT,
                            margin_right: "24px",
                            width: "15%",
                            height: "15%",
                        }
                        h2 {
                            margin_top: 0,
                            "Create a new Peregrine account"
                        }
                    }

                    if let Some(error_message) = error() {
                        div {
                            class: "error-container",
                            text_align: "center",
                            margin_top: "8px",
                            margin_bottom: "12px",
                            p { "{error_message}" }
                        }
                    }

                    br {}

                    form {
                        onsubmit: move |event| create_account(event, error),
                        p {
                            margin: 0,
                            margin_bottom: "8px",
                            "Email "
                            b {
                                color: "#b67de9",
                                padding: 0,
                                margin: 0,
                                "*"
                            }
                        }
                        input { name: "email", margin_top: "8px", maxlength: 254 }
                        br {}
                        br {}
                        p { margin: 0, "Username" }
                        input { name: "username", margin_top: "8px", maxlength: 32 }
                        br {}
                        br {}
                        p {
                            margin: 0,
                            margin_bottom: "8px",
                            "Password "
                            b {
                                color: "#b67de9",
                                padding: 0,
                                margin: 0,
                                "*"
                            }
                        }
                        input { name: "password", margin_top: "8px", r#type: "password" }
                        if advanced_mode() {
                            br {}
                            br {}
                            p {
                                margin: 0,
                                margin_bottom: "8px",
                                "Server"
                            }
                            input {
                                disabled: true,
                                name: "server",
                                margin_top: "9px",
                                placeholder: DEFAULT_SERVER,
                                value: last_entered_server(),
                                oninput: move |event| last_entered_server.set(event.value()),
                            }
                        }
                        br {}
                        br {}
                        br {}

                        button {
                            padding: "8px",
                            padding_left: "12px",
                            padding_right: "12px",
                            width: "100%",
                            "Create account",
                        }
                    }
                    br {}

                    p {
                        text_align: "center",
                        margin_bottom: "8px",
                        "Already have an account? "
                        Link { to: Route::LoginAccount {}, "Log in" }
                    }
                    if !advanced_mode() {
                        p {
                            text_align: "center",
                            margin_bottom: "8px",
                            "Looking for more advanced options? "
                            a { href: "", onclick: move |_| { advanced_mode.set(true); }, "Show them" }
                        }
                    } else {
                        p {
                            text_align: "center",
                            margin_bottom: "8px",
                            "Too scary? "
                            a { href: "", onclick: move |_| { advanced_mode.set(false); }, "Return back" }
                        }
                    }
                    br {}
                }
            }
        }
    }
}

#[component]
pub fn LoginAccount() -> Element {
    const PANEL_WIDTH: u32 = 480;
    const PANEL_MARGIN_WIDTH: u32 = 48;
    const PANEL_MARGIN_HEIGHT: u32 = 36;
    const INNER_PANEL_WIDTH: u32 = PANEL_WIDTH - PANEL_MARGIN_WIDTH * 2;
    const ICON_TRANSPARENT: Asset = asset!(
        "/assets/icon_transparent.png",
        ImageAssetOptions::new()
            .with_size(ImageSize::Manual {
                width: 97,
                height: 111,
            })
            .with_format(ImageFormat::Avif)
    );

    let error: Signal<Option<String>> = use_signal(|| None);
    let mut advanced_mode: Signal<bool> = use_signal(|| false);
    let mut last_entered_server: Signal<String> = use_signal(|| "".to_owned());

    async fn login_account(event: Event<FormData>, mut error_sig: Signal<Option<String>>) -> () {
        let values = event.values();
        let login: &str = &values["login"].as_value();
        let password: &str = &values["password"].as_value();
        let server: String = if values.contains_key("server") {
            let value = values["server"].as_value();
            if value.is_empty() {
                DEFAULT_SERVER.to_owned()
            } else {
                value
            }
        } else {
            DEFAULT_SERVER.to_owned()
        };

        if let Some(error) = check_password(password) {
            error_sig.set(Some(error.clone()));
            return;
        }
        if let Some(error) = check_server(&server) {
            error_sig.set(Some(error.clone()));
            return;
        }

        let mut cryptoset = shared::crypto::default_cryptoset(password.as_bytes(), None);
        let public_key = cryptoset.asymmetric_cipher.clone().into_public_key_bytes();
        let session_params = SessionParams {
            current_timestamp: chrono::Utc::now().timestamp().cast_unsigned(),
            authorize_before_seconds: LIMITS.max_session_before_period,
            authorize_after_seconds: LIMITS.max_session_after_period,
            session_validity_seconds: LIMITS.max_session_validity_period,
        };
        let session_params_bytes = session_params.to_boxed_slice();
        let signature = cryptoset.asymmetric_cipher.sign(&session_params_bytes, &mut cryptoset.rng);
        if !cryptoset.asymmetric_cipher.verify(&session_params_bytes, &signature) {
            error!("Failed to verify login signature on client-side, will probably be rejected by server.");
        }
        info!(
            "Submitting form: login='{login}', server='{server}', public_key={public_key:?}, session_params={session_params:?}, signature={signature:?}"
        );
        error_sig.set(None);

        let (account_id, session_token) = match server::login_account(
            login.to_owned(),
            crypto::preferred_alogirthm().to_owned(),
            public_key,
            session_params,
            signature,
        ).await {
            Ok(value) => value,
            Err(err) => {
                eprintln!("Error while trying to log into account: {err:?}");
                error_sig.set(Some("Invalid login or password".to_owned()));
                return;
            }
        };
        let login_credentials = AccountCredentials {
            id: account_id,
            session_token,
        };
        STORAGE.store_session_credentials(login_credentials);
        let nav = navigator();
        nav.replace(Route::Contacts {
            credentials: login_credentials,
        });
        info!("Form submitted, session token: {session_token:?}");
    }

    rsx! {
        div {
            id: "center-container",

            div {
                id: "main-panel",
                class: "panel noselect",
                width: format!("{PANEL_WIDTH}px"),
                max_height: "94vh",

                div {
                    id: "inside-container",
                    margin_left: format!("{PANEL_MARGIN_WIDTH}px"),
                    width: format!("{INNER_PANEL_WIDTH}px"),
                    margin_top: format!("{PANEL_MARGIN_HEIGHT}px"),

                    div {
                        display: "flex",
                        margin_left: "46px",
                        margin_right: "32px",
                        img {
                            src: ICON_TRANSPARENT,
                            margin_right: "24px",
                            width: "15%",
                            height: "15%",
                        }
                        h2 {
                            margin_top: 0,
                            "Log into an existing account"
                        }
                    }

                    if let Some(error_message) = error() {
                        div {
                            class: "error-container",
                            text_align: "center",
                            margin_top: "8px",
                            margin_bottom: "12px",
                            p { "{error_message}" }
                        }
                    }

                    br {}

                    form {
                        onsubmit: move |event| login_account(event, error),
                        p {
                            margin: 0,
                            margin_bottom: "8px",
                            "Username or email "
                            b {
                                color: "#b67de9",
                                padding: 0,
                                margin: 0,
                                "*"
                            }
                        }
                        input { name: "login", margin_top: "8px", maxlength: 254 }
                        br {}
                        br {}
                        p {
                            margin: 0,
                            margin_bottom: "8px",
                            "Password "
                            b {
                                color: "#b67de9",
                                padding: 0,
                                margin: 0,
                                "*"
                            }
                        }
                        input { name: "password", margin_top: "8px", r#type: "password" }
                        if advanced_mode() {
                            br {}
                            br {}
                            p {
                                margin: 0,
                                margin_bottom: "8px",
                                "Server"
                            }
                            input {
                                disabled: true,
                                name: "server",
                                margin_top: "9px",
                                placeholder: DEFAULT_SERVER,
                                value: last_entered_server(),
                                oninput: move |event| last_entered_server.set(event.value()),
                            }
                        }
                        br {}
                        br {}
                        br {}

                        button {
                            padding: "8px",
                            padding_left: "12px",
                            padding_right: "12px",
                            width: "100%",
                            "Log into an account",
                        }
                    }
                    br {}

                    p {
                        text_align: "center",
                        margin_bottom: "8px",
                        "Don't have an account? "
                        Link { to: Route::RegisterAccount {}, "Sign up" }
                    }
                    if !advanced_mode() {
                        p {
                            text_align: "center",
                            margin_bottom: "8px",
                            "Looking for more advanced options? "
                            a { href: "", onclick: move |_| { advanced_mode.set(true); }, "Show them" }
                        }
                    } else {
                        p {
                            text_align: "center",
                            margin_bottom: "8px",
                            "Too scary? "
                            a { href: "", onclick: move |_| { advanced_mode.set(false); }, "Return back" }
                        }
                    }
                    br {}
                }
            }
        }
    }
}
