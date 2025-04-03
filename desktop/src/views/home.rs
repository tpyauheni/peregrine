use dioxus::prelude::*;
use ui::{Echo, Hero};

use crate::Route;

#[component]
pub fn Home() -> Element {
    let logged_in: bool = false;

    if !logged_in {
        let nav = navigator();
        nav.replace(Route::RegisterAccount {});
    }

    rsx! {
        Hero {}
        Echo {}
    }
}
