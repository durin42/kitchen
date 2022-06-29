// Copyright 2022 Jeremy Wall (jeremy@marzhillstudios.com)
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
use crate::components::{recipe::Recipe, tabs::*};
use crate::pages::PageState;

use sycamore::prelude::*;

pub struct RecipePageProps {
    pub page_state: PageState,
    pub recipe: Signal<usize>,
}

#[component(RecipePage<G>)]
pub fn recipe_page(props: RecipePageProps) -> View<G> {
    view! {
        TabbedView(TabState {
            route: props.page_state.route.clone(),
            inner: view! {
                Recipe(props.recipe.handle())
            }
        })
    }
}
