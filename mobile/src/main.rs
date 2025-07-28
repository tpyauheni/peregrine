use dioxus::{logger::tracing::Level, prelude::*};

use ::server::*;
use views::{
    Contacts, CreateGroup, GroupMenu, Home, Invites, LoginAccount, OtherUserAccount,
    RegisterAccount, SessionValidityChecker,
};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(MobileNavbar)]
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
    #[route("/user?:user_id&:credentials")]
    OtherUserAccount { user_id: u64, credentials: AccountCredentials },
    #[route("/create_group/:credentials")]
    CreateGroup { credentials: AccountCredentials },
    #[route("/group?:group_id&:credentials")]
    GroupMenu { group_id: u64, credentials: AccountCredentials },
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    #[cfg(debug_assertions)]
    {
        dioxus::logger::init(Level::DEBUG).unwrap();
    }
    #[cfg(not(debug_assertions))]
    {
        dioxus::logger::init(Level::INFO).unwrap();
    }
    // I'm no really sure the following block is useful for mobile:
    #[cfg(all(not(feature = "mobile"), feature = "server"))]
    {
        init_server();
    }

<<<<<<< Updated upstream
    #[cfg(all(feature = "mobile", not(debug_assertions)))]
    {
        use dioxus::desktop::Config;
        use dioxus::desktop::WindowBuilder;

        dioxus::LaunchBuilder::new()
            .with_cfg(
                Config::default().with_menu(None).with_window(
                    WindowBuilder::new()
                        .with_fullscreen(true)
                        .with_title("Peregrine"),
                ),
            )
            .launch(App);
    }
    #[cfg(not(all(feature = "mobile", not(debug_assertions))))]
=======
>>>>>>> Stashed changes
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    #[cfg(feature = "server")]
    init_server();
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

#[component]
fn MobileNavbar() -> Element {
    rsx! {
        Outlet::<Route> {}
    }
}
