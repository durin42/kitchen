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
use crate::components::recipe::Viewer;

use sycamore::prelude::*;
use tracing::instrument;

use super::{RecipePage, RecipePageProps};

#[instrument(skip_all, fields(recipe=props.recipe))]
#[component()]
pub fn RecipeViewPage<G: Html>(cx: Scope, props: RecipePageProps) -> View<G> {
    let RecipePageProps { recipe, sh } = props;
    view! {cx,
        RecipePage(
            selected=Some("View".to_owned()),
            recipe=recipe.clone(),
        ) { Viewer(recipe_id=recipe, sh=sh) }
    }
}
