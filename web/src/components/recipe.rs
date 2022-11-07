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
use tracing::{debug, error};
use web_sys::HtmlDialogElement;

use crate::{app_state, js_lib::get_element_by_id};
use recipes::{self, RecipeEntry};

fn get_error_dialog() -> HtmlDialogElement {
    get_element_by_id::<HtmlDialogElement>("error-dialog")
        .expect("error-dialog isn't an html dialog element!")
        .expect("error-dialog element isn't present")
}

fn check_recipe_parses(text: &str, error_text: &Signal<String>) -> bool {
    if let Err(e) = recipes::parse::as_recipe(text) {
        error!(?e, "Error parsing recipe");
        error_text.set(e);
        let el = get_error_dialog();
        el.show();
        false
    } else {
        error_text.set(String::new());
        let el = get_error_dialog();
        el.close();
        true
    }
}

#[component]
pub fn Editor<G: Html>(cx: Scope, recipe_id: String) -> View<G> {
    let store = crate::api::HttpStore::get_from_context(cx);
    let recipe: &Signal<RecipeEntry> =
        create_signal(cx, RecipeEntry::new(&recipe_id, String::new()));
    let text = create_signal(cx, String::new());
    spawn_local_scoped(cx, {
        let store = store.clone();
        async move {
            let entry = store
                .get_recipe_text(recipe_id.as_str())
                .await
                .expect("Failure getting recipe");
            if let Some(entry) = entry {
                text.set(entry.recipe_text().to_owned());
                recipe.set(entry);
            } else {
                // FIXME(jwall): Show error message for missing recipe
            }
        }
    });

    let id = create_memo(cx, || recipe.get().recipe_id().to_owned());
    let error_text = create_signal(cx, String::new());
    let save_signal = create_signal(cx, ());
    let dirty = create_signal(cx, false);

    debug!("Creating effect");
    create_effect(cx, move || {
        save_signal.track();
        if !*dirty.get_untracked() {
            debug!("Recipe text is unchanged");
            return;
        }
        debug!("Recipe text is changed");
        spawn_local_scoped(cx, {
            let store = crate::api::HttpStore::get_from_context(cx);
            let state = app_state::State::get_from_context(cx);
            async move {
                debug!("Attempting to save recipe");
                if let Err(e) = store
                    .save_recipes(vec![RecipeEntry(
                        id.get_untracked().as_ref().clone(),
                        text.get_untracked().as_ref().clone(),
                    )])
                    .await
                {
                    error!(?e, "Failed to save recipe");
                    error_text.set(format!("{:?}", e));
                } else {
                    // We also need to set recipe in our state
                    dirty.set(false);
                    if let Ok(recipe) = recipes::parse::as_recipe(text.get_untracked().as_ref()) {
                        state
                            .recipes
                            .modify()
                            .insert(id.get_untracked().as_ref().to_owned(), recipe);
                    }
                };
            }
        });
    });

    debug!("creating dialog_view");
    let dialog_view = view! {cx,
        dialog(id="error-dialog") {
            article{
                header {
                    a(href="#", on:click=|_| {
                        let el = get_error_dialog();
                        el.close();
                    }, class="close")
                    "Invalid Recipe"
                }
                p {
                    (error_text.get().clone())
                }
            }
        }
    };

    debug!("creating editor view");
    view! {cx,
        (dialog_view)
        textarea(bind:value=text, rows=20, on:change=move |_| {
            dirty.set(true);
        })
        span(role="button", on:click=move |_| {
            let unparsed = text.get();
            check_recipe_parses(unparsed.as_str(), error_text.clone());
        }) { "Check" } " "
        span(role="button", on:click=move |_| {
            let unparsed = text.get();
            if check_recipe_parses(unparsed.as_str(), error_text.clone()) {
                debug!("triggering a save");
                save_signal.trigger_subscribers();
            };
        }) { "Save" }
    }
}

#[component]
fn Steps<G: Html>(cx: Scope, steps: Vec<recipes::Step>) -> View<G> {
    let step_fragments = View::new_fragment(steps.iter().map(|step| {
        let mut step = step.clone();
        let ingredient_fragments = View::new_fragment(step.ingredients.drain(0..).map(|i| {
            view! {cx,
                li {
                    (i.amt) " " (i.name) " " (i.form.as_ref().map(|f| format!("({})", f)).unwrap_or(String::new()))
                }
            }
        }).collect());
        view! {cx,
            div {
                h3 { "Instructions" }
                ul(class="ingredients") {
                    (ingredient_fragments)
                }
                div(class="instructions") {
                    (step.instructions)
                }
            }
        }
    }).collect());
    view! {cx,
            h2 { "Steps: " }
            div(class="recipe_steps") {
                (step_fragments)
            }
    }
}

#[component]
pub fn Viewer<G: Html>(cx: Scope, recipe_id: String) -> View<G> {
    let state = app_state::State::get_from_context(cx);
    let view = create_signal(cx, View::empty());
    if let Some(recipe) = state.recipes.get_untracked().get(&recipe_id) {
        let title = recipe.title.clone();
        let desc = recipe.desc.clone().unwrap_or_else(|| String::new());
        let steps = recipe.steps.clone();
        debug!("Viewing recipe.");
        view.set(view! {cx,
            div(class="recipe") {
                h1(class="recipe_title") { (title) }
                 div(class="recipe_description") {
                     (desc)
                 }
                Steps(steps)
            }
        });
    }
    view! {cx, (view.get().as_ref()) }
}

//#[component]
//pub fn Recipe<'ctx, G: Html>(cx: Scope<'ctx>, recipe_id: String) -> View<G> {
//    let state = app_state::State::get_from_context(cx);
//    let store = crate::api::HttpStore::get_from_context(cx);
//    let view = create_signal(cx, View::empty());
//    let show_edit = create_signal(cx, false);
//    if let Some(recipe) = state.recipes.get_untracked().get(&recipe_id) {
//        // FIXME(jwall): This should be create_effect rather than create_signal
//        let recipe_text: &Signal<Option<RecipeEntry>> = create_signal(cx, None);
//        spawn_local_scoped(cx, {
//            let store = store.clone();
//            async move {
//                let entry = store
//                    .get_recipe_text(recipe_id.as_str())
//                    .await
//                    .expect("Failure getting recipe");
//                recipe_text.set(entry);
//            }
//        });
//        let recipe = create_signal(cx, recipe.clone());
//        let title = create_memo(cx, move || recipe.get().title.clone());
//        let desc = create_memo(cx, move || {
//            recipe
//                .clone()
//                .get()
//                .desc
//                .clone()
//                .unwrap_or_else(|| String::new())
//        });
//        let steps = create_memo(cx, move || recipe.get().steps.clone());
//        create_effect(cx, move || {
//            debug!("Choosing edit or view for recipe.");
//            if *show_edit.get() {
//                {
//                    debug!("Showing editor for recipe.");
//                    view.set(view! {cx,
//                        Editor(recipe_text.get().as_ref().clone().unwrap())
//                    });
//                }
//            } else {
//                debug!("Showing text for recipe.");
//                view.set(view! {cx,
//                    div(class="recipe") {
//                        h1(class="recipe_title") { (title.get()) }
//                         div(class="recipe_description") {
//                             (desc.get())
//                         }
//                        Steps(steps)
//                    }
//                });
//            }
//        });
//    }
//    view! {cx,
//        span(role="button", on:click=move |_| { show_edit.set(true); }) { "Edit" } " "
//        span(role="button", on:click=move |_| { show_edit.set(false); }) { "View" }
//        (view.get().as_ref())
//    }
//}
