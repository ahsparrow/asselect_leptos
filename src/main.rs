// Copyright 2024, Alan Sparrow
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or (at
// your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.
//
use codee::string::JsonSerdeCodec;
use futures::join;
use gloo::file::{Blob, ObjectUrl};
use gloo::net::http::Request;
use leptos::html::{p, A};
use leptos::prelude::*;
use leptos::web_sys;
use leptos_use::storage::use_local_storage;

use components::{
    about_tab::AboutTab, airspace_tab::AirspaceTab, extra_panel::ExtraPanel, extra_tab::ExtraTab,
    notam_tab::NotamTab, option_tab::OptionTab, tabs::Tabs,
};
use convert::openair;
use settings::{ExtraType, Overlay, Settings};
use yaixm::{gliding_sites, loa_names, rat_names, wave_names, Yaixm};

mod components;
mod convert;
mod settings;
mod yaixm;

#[derive(Clone, Debug)]
struct OverlayData {
    overlay_195: Option<String>,
    overlay_105: Option<String>,
    overlay_atzdz: Option<String>,
}

#[component]
fn App() -> impl IntoView {
    let async_yaixm = LocalResource::new(fetch_yaixm);

    let async_overlay = LocalResource::new(|| async {
        let overlay_195 = fetch_overlay("overlay_195.txt");
        let overlay_105 = fetch_overlay("overlay_105.txt");
        let overlay_atzdz = fetch_overlay("overlay_atzdz.txt");
        let (o_195, o_105, o_atzdz) = join!(overlay_195, overlay_105, overlay_atzdz);
        OverlayData {
            overlay_195: o_195,
            overlay_105: o_105,
            overlay_atzdz: o_atzdz,
        }
    });

    move || match async_yaixm.get().as_deref() {
        Some(resource) => match resource {
            Some(yaixm) => {
                view! { <MainView yaixm=yaixm.clone() overlay=async_overlay /> }
            }
            .into_any(),
            None => p().child("Error getting airspace data").into_any(),
        },
        None => p()
            .child("Getting airspace data, please wait...")
            .into_any(),
    }
}

#[component]
fn MainView(yaixm: Yaixm, overlay: LocalResource<OverlayData>) -> impl IntoView {
    // Local settings storage
    let (local_settings, set_local_settings, _) =
        use_local_storage::<Settings, JsonSerdeCodec>("settings");

    // Make copy of settings so store value is only updated on download
    let (settings, set_settings) = signal(local_settings.get_untracked());
    provide_context(settings);
    provide_context(set_settings);

    // Release note modal display control
    let (modal, set_modal) = signal(false);

    // UI data from YAIXM
    let rat_names = rat_names(&yaixm);
    let mut loa_names = loa_names(&yaixm);
    let mut wave_names = wave_names(&yaixm);
    loa_names.sort();
    wave_names.sort();

    let mut gliding_sites = gliding_sites(&yaixm);
    gliding_sites.sort();

    let airac_date = yaixm.release.airac_date[..10].to_string();
    let release_note = yaixm.release.note.clone();

    // UI static data
    let tab_names = vec![
        "Main".to_string(),
        "Option".to_string(),
        "Extra".to_string(),
        "NOTAM".to_string(),
        "About".to_string(),
    ];

    let extra_names = vec![
        "Temporary Restrictions".to_string(),
        "Local Agreements".to_string(),
        "Wave Boxes".to_string(),
    ];

    let extra_ids = vec![ExtraType::Rat, ExtraType::Loa, ExtraType::Wave];

    let download_node_ref = NodeRef::<A>::new();

    // Download button callback
    let download = move |_| {
        // Store settings
        set_local_settings.set(settings.get_untracked());

        let user_agent = web_sys::window()
            .and_then(|w| w.navigator().user_agent().ok())
            .unwrap_or_default();

        // Create OpenAir data
        let oa = openair(&yaixm, &settings.get_untracked(), &user_agent);

        // Get overlay data
        let od = if let Some(overlay_setting) = settings().overlay {
            if let Some(overlay_data) = overlay.get().as_deref() {
                let x = match overlay_setting {
                    Overlay::FL195 => overlay_data.clone().overlay_195,
                    Overlay::FL105 => overlay_data.clone().overlay_105,
                    Overlay::AtzDz => overlay_data.clone().overlay_atzdz,
                };
                x.unwrap_or("* Missing overlay data".to_string())
            } else {
                "* Overlay data not loaded".to_string()
            }
        } else {
            "".to_string()
        };

        // Create download data
        let blob = Blob::new((oa + od.as_str()).as_str());
        let object_url = ObjectUrl::from(blob);

        let a = download_node_ref.get().unwrap();
        a.set_download("openair.txt");
        a.set_href(&object_url);
        a.click();
    };

    view! {
        <header class="hero is-small is-primary block">
            <div class="hero-body">
                <div class="container">
                    <div class="title is-4">{"ASSelect - UK Airspace"}</div>
                </div>
            </div>
        </header>

        <div class="container block">
            <Tabs tab_names>
                <AirspaceTab gliding_sites=gliding_sites/>
                <OptionTab/>
                <ExtraTab names=extra_names ids=extra_ids>
                    <ExtraPanel names=rat_names id=ExtraType::Rat/>
                    <ExtraPanel names=loa_names id=ExtraType::Loa/>
                    <ExtraPanel names=wave_names id=ExtraType::Wave/>
                </ExtraTab>
                <NotamTab/>
                <AboutTab/>
            </Tabs>
        </div>

        <div class="container block">
            <div class="mx-4">
                <button type="submit" class="button is-primary" on:click=download>
                    {"Get Airspace"}
                </button>

                <a id="airac-button" class="button is-text is-pulled-right" on:click=move |_| set_modal(true)>
                    "AIRAC: "{ airac_date }
                </a>
            </div>
        </div>

        // Release note overlay
        <div class="modal" class:is-active=modal>
            <div class="modal-background"></div>
                <div class="modal-content">
                    <div class="box">
                        <h2 class="subtitle">{"Release Details"}</h2>
                        <pre>{ release_note }</pre>
                    </div>
                </div>
            <button class="modal-close is-large" on:click=move |_| set_modal(false)></button>
        </div>

        // For data download
        <a hidden node_ref=download_node_ref></a>
    }
}

// Get YAIXM data from server
async fn fetch_yaixm() -> Option<Yaixm> {
    let result = Request::get("yaixm.json").send().await;
    match result {
        Ok(response) => response.json().await.ok(),
        _ => None,
    }
}

// Get overlay data from server
async fn fetch_overlay(path: &str) -> Option<String> {
    let result = Request::get(path).send().await;
    match result {
        Ok(response) => response.text().await.ok(),
        _ => None,
    }
}

fn main() {
    console_error_panic_hook::set_once();
    mount_to_body(|| view! { <App/> })
}
