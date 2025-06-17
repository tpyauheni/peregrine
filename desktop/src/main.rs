use dioxus::prelude::*;

use server::AccountCredentials;
use views::{Contacts, Home, LoginAccount, RegisterAccount, SessionValidityChecker, Invites};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(DesktopNavbar)]
    #[route("/")]
    Home {},
    #[route("/contacts/:credentials")]
    Contacts { credentials: AccountCredentials },
    #[end_layout]
    #[nest("/account")]
        #[route("/")]
        RegisterAccount {},
        #[route("/signup")]
        LoginAccount {},
    #[end_nest]
    #[route("/check_session/:credentials")]
    SessionValidityChecker { credentials: AccountCredentials },
    #[route("/invites/:credentials")]
    Invites { credentials: AccountCredentials },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    #[cfg(all(feature = "desktop", not(debug_assertions)))]
    {
        use dioxus::desktop::Config;
        use dioxus::desktop::WindowBuilder;

        dioxus::LaunchBuilder::new()
            .with_cfg(
                Config::default().with_menu(None).with_window(
                    WindowBuilder::new()
                        .with_maximized(true)
                        .with_title("Peregrine"),
                ),
            )
            .launch(App);
    }
    #[cfg(all(not(feature = "desktop"), feature = "server"))]
    {
        server::init_server();
    }

    #[cfg(any(not(feature = "desktop"), debug_assertions))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    #[cfg(feature = "server")]
    server::init_server();
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
fn DesktopNavbar() -> Element {
    rsx! {
        Outlet::<Route> {}
    }
}
