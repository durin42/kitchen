// Copyright 2022 Jeremy Wall
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.
use sycamore::{futures::spawn_local_scoped, prelude::*};
use tracing::{info, instrument};

use crate::app_state::Message;
use crate::components::{Footer, Header};
use crate::{api, routing::Handler as RouteHandler};

#[instrument]
#[component]
pub fn UI<G: Html>(cx: Scope) -> View<G> {
    api::HttpStore::provide_context(cx, "/api".to_owned());
    // FIXME(jwall): We shouldn't need to get the store from a context anymore.
    let store = api::HttpStore::get_from_context(cx).as_ref().clone();
    info!("Starting UI");
    let app_state = crate::app_state::AppState::new();
    let sh = crate::app_state::get_state_handler(cx, app_state, store);
    let view = create_signal(cx, View::empty());
    // FIXME(jwall): We need a way to trigger refreshes when required. Turn this
    // into a create_effect with a refresh signal stored as a context.
    spawn_local_scoped(cx, {
        let store = api::HttpStore::get_from_context(cx);
        async move {
            sh.dispatch(cx, Message::LoadState);
            // TODO(jwall): This needs to be moved into the RouteHandler
            view.set(view! { cx,
                div(class="app") {
                    Header(sh)
                    RouteHandler(sh=sh)
                    Footer { }
                }
            });
        }
    });

    view! { cx, (view.get().as_ref()) }
}
