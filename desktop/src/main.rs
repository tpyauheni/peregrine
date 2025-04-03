use dioxus::prelude::*;

use ui::Navbar;
use views::{Blog, Home, RegisterAccount, LoginAccount};

mod views;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(DesktopNavbar)]
    #[route("/")]
    Home {},
    #[route("/blog/:id")]
    Blog { id: i32 },
    #[end_layout]
    #[nest("/account")]
        #[route("/")]
        RegisterAccount {},
        #[route("/signup")]
        LoginAccount {},
}

const MAIN_CSS: Asset = asset!("/assets/main.css");

fn main() {
    // TODO: Check if logged into account, then:
    // TODO: Otherwise, show create account menu
    // #[cfg(feature = "desktop")]
    // {
    //     use dioxus::desktop::Config;
    //     use dioxus::desktop::WindowBuilder;
    //
    //     dioxus::LaunchBuilder::new()
    //         .with_cfg(
    //             Config::default()
    //                 .with_menu(None)
    //                 .with_window(
    //                     WindowBuilder::new()
    //                         .with_maximized(true)
    //                         .with_title("Peregrine")
    //                 )
    //             )
    //         .launch(App);
    // }
    // #[cfg(not(feature = "desktop"))]
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    rsx! {
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        Router::<Route> {}
    }
}

/// A desktop-specific Router around the shared `Navbar` component
/// which allows us to use the desktop-specific `Route` enum.
#[component]
fn DesktopNavbar() -> Element {
    rsx! {
        Navbar {
            Link {
                to: Route::Home {},
                "Home"
            }
            Link {
                to: Route::Blog { id: 1 },
                "Blog"
            }
        }

        Outlet::<Route> {}
    }
}
