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
use sycamore::prelude::*;

use crate::app_state::AppRoutes;

#[derive(Clone)]
pub struct TabState<G: GenericNode> {
    pub route: Signal<AppRoutes>,
    pub inner: View<G>,
}

#[component(TabbedView<G>)]
pub fn tabbed_view(state: TabState<G>) -> View<G> {
    cloned!((state) => view! {
        div(class="nav-header no-print") {
            a(class="no-print", on:click=cloned!((state) => move |_| {
                state.route.set(AppRoutes::Plan);
            })) { "Plan" }
            " | "
            a(class="no-print", on:click=cloned!((state) => move |_| {
                state.route.set(AppRoutes::Inventory);
            })) { "Inventory" }
            " | "
            a(on:click=cloned!((state) => move |_| {
                state.route.set(AppRoutes::Cook);
            })) { "Cook" }
        }
        div {
            (state.inner)
        }
    })
}