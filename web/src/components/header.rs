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

#[component]
pub fn Header<G: Html>(cx: Scope) -> View<G> {
    view! {cx,
        nav(class="no-print") {
            h1(class="title") { "Kitchen" }
            ul {
                li { a(href="/ui/planning/plan") { "MealPlan" } }
                li { a(href="/ui/manage/categories") { "Manage" } }
                li { a(href="/ui/login") { "Login" } }
                li { a(href="https://github.com/zaphar/kitchen") { "Github" } }
            }
        }
    }
}
