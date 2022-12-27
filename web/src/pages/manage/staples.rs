// Copyright 2022 Jeremy Wall (Jeremy@marzhilsltudios.com)
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
use super::ManagePage;
use crate::{app_state::StateHandler, components::recipe::Editor};

use sycamore::prelude::*;
use tracing::instrument;

#[instrument(skip_all)]
#[component()]
pub fn StaplesPage<'ctx, G: Html>(cx: Scope<'ctx>, sh: StateHandler<'ctx>) -> View<G> {
    view! {cx,
        ManagePage(
            selected=Some("Staples".to_owned()),
        ) { Editor(recipe_id="staples.txt".to_owned(), sh=sh) }
    }
}
