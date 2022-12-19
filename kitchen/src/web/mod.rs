use std::collections::BTreeMap;
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
use std::path::PathBuf;
use std::sync::Arc;
use std::{collections::BTreeSet, net::SocketAddr};

use axum::{
    body::{boxed, Full},
    extract::{Extension, Json, Path},
    http::{header, StatusCode},
    response::{IntoResponse, Redirect, Response},
    routing::{get, Router},
};
use chrono::NaiveDate;
use mime_guess;
use recipes::{IngredientKey, RecipeEntry};
use rust_embed::RustEmbed;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;
use tracing::{debug, info, instrument};

use api;
use storage::{APIStore, AuthStore};

mod auth;
mod storage;

#[derive(RustEmbed)]
#[folder = "../web/dist"]
struct UiAssets;

pub struct StaticFile<T>(pub T);

impl<T> IntoResponse for StaticFile<T>
where
    T: Into<String>,
{
    fn into_response(self) -> Response {
        let path = self.0.into();

        match UiAssets::get(path.as_str()) {
            Some(content) => {
                let body = boxed(Full::from(content.data));
                let mime = mime_guess::from_path(path).first_or_octet_stream();
                Response::builder()
                    .header(header::CONTENT_TYPE, mime.as_ref())
                    .body(body)
                    .unwrap()
            }
            None => Response::builder()
                .status(StatusCode::NOT_FOUND)
                .body(boxed(Full::from("404")))
                .unwrap(),
        }
    }
}

#[instrument]
async fn ui_static_assets(Path(path): Path<String>) -> impl IntoResponse {
    info!("Serving ui path");

    let mut path = path.trim_start_matches("/");
    if UiAssets::get(path).is_none() {
        path = "index.html";
    }
    debug!(path = path, "Serving transformed path");
    StaticFile(path.to_owned())
}

#[instrument]
async fn api_recipe_entry(
    Extension(store): Extension<Arc<storage::file_store::AsyncFileStore>>,
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
    Path(recipe_id): Path<String>,
) -> api::Response<Option<RecipeEntry>> {
    use storage::{UserId, UserIdFromSession::*};
    let result = match session {
        NoUserId => store
            .get_recipe_entry(recipe_id)
            .await
            .map_err(|e| format!("Error: {:?}", e)),
        FoundUserId(UserId(id)) => app_store
            .get_recipe_entry_for_user(id, recipe_id)
            .await
            .map_err(|e| format!("Error: {:?}", e)),
    };
    result.into()
}

#[instrument]
async fn api_recipes(
    Extension(store): Extension<Arc<storage::file_store::AsyncFileStore>>,
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
) -> api::RecipeEntryResponse {
    // Select recipes based on the user-id if it exists or serve the default if it does not.
    use storage::{UserId, UserIdFromSession::*};
    let result = match session {
        NoUserId => store
            .get_recipes()
            .await
            .map_err(|e| format!("Error: {:?}", e)),
        FoundUserId(UserId(id)) => app_store
            .get_recipes_for_user(id.as_str())
            .await
            .map_err(|e| format!("Error: {:?}", e)),
    };
    result.into()
}

#[instrument]
async fn api_categories(
    Extension(store): Extension<Arc<storage::file_store::AsyncFileStore>>,
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
) -> api::Response<String> {
    // Select Categories based on the user-id if it exists or serve the default if it does not.
    use storage::{UserId, UserIdFromSession::*};
    let categories_result = match session {
        NoUserId => store
            .get_categories()
            .await
            .map_err(|e| format!("Error: {:?}", e)),
        FoundUserId(UserId(id)) => app_store
            .get_categories_for_user(id.as_str())
            .await
            .map_err(|e| format!("Error: {:?}", e)),
    };
    categories_result.into()
}

async fn api_save_categories(
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
    Json(categories): Json<String>,
) -> api::Response<String> {
    use storage::{UserId, UserIdFromSession::FoundUserId};
    if let FoundUserId(UserId(id)) = session {
        if let Err(e) = app_store
            .store_categories_for_user(id.as_str(), categories.as_str())
            .await
        {
            return api::Response::error(
                StatusCode::INTERNAL_SERVER_ERROR.as_u16(),
                format!("{:?}", e),
            );
        }
        api::Response::success("Successfully saved categories".into())
    } else {
        api::Response::Unauthorized
    }
}

async fn api_save_recipes(
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
    Json(recipes): Json<Vec<RecipeEntry>>,
) -> api::Response<()> {
    use storage::{UserId, UserIdFromSession::FoundUserId};
    if let FoundUserId(UserId(id)) = session {
        let result = app_store
            .store_recipes_for_user(id.as_str(), &recipes)
            .await;
        result.map_err(|e| format!("Error: {:?}", e)).into()
    } else {
        api::Response::Unauthorized
    }
}

async fn api_plan(
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
) -> api::PlanDataResponse {
    use storage::{UserId, UserIdFromSession::FoundUserId};
    if let FoundUserId(UserId(id)) = session {
        app_store
            .fetch_latest_meal_plan(&id)
            .await
            .map_err(|e| format!("Error: {:?}", e))
            .into()
    } else {
        api::Response::Unauthorized
    }
}

async fn api_plan_since(
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
    Path(date): Path<chrono::NaiveDate>,
) -> api::Response<BTreeMap<NaiveDate, Vec<(String, i32)>>> {
    use storage::{UserId, UserIdFromSession::FoundUserId};
    if let FoundUserId(UserId(id)) = session {
        app_store
            .fetch_meal_plans_since(&id, date)
            .await
            .map_err(|e| format!("Error: {:?}", e))
            .into()
    } else {
        api::Response::Unauthorized
    }
}

async fn api_save_plan(
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
    Json(meal_plan): Json<Vec<(String, i32)>>,
) -> api::Response<()> {
    use storage::{UserId, UserIdFromSession::FoundUserId};
    if let FoundUserId(UserId(id)) = session {
        app_store
            .save_meal_plan(id.as_str(), &meal_plan, chrono::Local::now().date_naive())
            .await
            .map_err(|e| format!("{:?}", e))
            .into()
    } else {
        api::Response::Unauthorized
    }
}

async fn api_inventory_v2(
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
) -> api::InventoryResponse {
    use storage::{UserId, UserIdFromSession::FoundUserId};
    if let FoundUserId(UserId(id)) = session {
        app_store
            .fetch_latest_inventory_data(id)
            .await
            .map_err(|e| format!("{:?}", e))
            .map(|d| {
                let data: api::InventoryData = d.into();
                data
            })
            .into()
    } else {
        api::Response::Unauthorized
    }
}

async fn api_inventory(
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
) -> api::Response<(Vec<IngredientKey>, Vec<(IngredientKey, String)>)> {
    use storage::{UserId, UserIdFromSession::FoundUserId};
    if let FoundUserId(UserId(id)) = session {
        app_store
            .fetch_latest_inventory_data(id)
            .await
            .map_err(|e| format!("{:?}", e))
            .map(|(filtered, modified, _)| (filtered, modified))
            .into()
    } else {
        api::Response::Unauthorized
    }
}

async fn save_inventory_data(
    app_store: Arc<storage::SqliteStore>,
    id: String,
    filtered_ingredients: BTreeSet<IngredientKey>,
    modified_amts: BTreeMap<IngredientKey, String>,
    extra_items: Vec<(String, String)>,
) -> api::Response<()> {
    app_store
        .save_inventory_data(id, filtered_ingredients, modified_amts, extra_items)
        .await
        .map_err(|e| format!("{:?}", e))
        .into()
}

async fn api_save_inventory_v2(
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
    Json((filtered_ingredients, modified_amts, extra_items)): Json<(
        Vec<IngredientKey>,
        Vec<(IngredientKey, String)>,
        Vec<(String, String)>,
    )>,
) -> api::Response<()> {
    use storage::{UserId, UserIdFromSession::FoundUserId};
    if let FoundUserId(UserId(id)) = session {
        let filtered_ingredients = filtered_ingredients.into_iter().collect();
        let modified_amts = modified_amts.into_iter().collect();
        save_inventory_data(
            app_store,
            id,
            filtered_ingredients,
            modified_amts,
            extra_items,
        )
        .await
        .into()
    } else {
        api::Response::Unauthorized
    }
}

async fn api_save_inventory(
    Extension(app_store): Extension<Arc<storage::SqliteStore>>,
    session: storage::UserIdFromSession,
    Json((filtered_ingredients, modified_amts)): Json<(
        Vec<IngredientKey>,
        Vec<(IngredientKey, String)>,
    )>,
) -> api::Response<()> {
    use storage::{UserId, UserIdFromSession::FoundUserId};
    if let FoundUserId(UserId(id)) = session {
        let filtered_ingredients = filtered_ingredients.into_iter().collect();
        let modified_amts = modified_amts.into_iter().collect();
        save_inventory_data(
            app_store,
            id,
            filtered_ingredients,
            modified_amts,
            Vec::new(),
        )
        .await
        .into()
    } else {
        api::Response::Unauthorized
    }
}

fn mk_v1_routes() -> Router {
    Router::new()
        .route("/recipes", get(api_recipes).post(api_save_recipes))
        // recipe entry api path route
        .route("/recipe/:recipe_id", get(api_recipe_entry))
        // mealplan api path routes
        .route("/plan", get(api_plan).post(api_save_plan))
        .route("/plan/:date", get(api_plan_since))
        // Inventory api path route
        .route("/inventory", get(api_inventory).post(api_save_inventory))
        .route("/categories", get(api_categories).post(api_save_categories))
        // All the routes above require a UserId.
        .route("/auth", get(auth::handler).post(auth::handler))
}

fn mk_v2_routes() -> Router {
    Router::new().route(
        "/inventory",
        get(api_inventory_v2).post(api_save_inventory_v2),
    )
}

#[instrument(fields(recipe_dir=?recipe_dir_path,listen=?listen_socket), skip_all)]
pub async fn ui_main(recipe_dir_path: PathBuf, store_path: PathBuf, listen_socket: SocketAddr) {
    let store = Arc::new(storage::file_store::AsyncFileStore::new(
        recipe_dir_path.clone(),
    ));
    let app_store = Arc::new(
        storage::SqliteStore::new(store_path)
            .await
            .expect("Unable to create app_store"),
    );
    app_store
        .run_migrations()
        .await
        .expect("Failed to run database migrations");
    let router = Router::new()
        .route("/", get(|| async { Redirect::temporary("/ui/plan") }))
        .route("/ui/*path", get(ui_static_assets))
        // TODO(jwall): We should use route_layer to enforce the authorization
        // requirements here.
        .nest(
            "/api",
            Router::new()
                .nest("/v1", mk_v1_routes())
                .nest("/v2", mk_v2_routes()),
        )
        // NOTE(jwall): Note that the layers are applied to the preceding routes not
        // the following routes.
        .layer(
            // NOTE(jwall): However service builder will apply the layers from top
            // to bottom.
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(Extension(store))
                .layer(Extension(app_store)),
        );
    info!(
        http = format!("http://{}", listen_socket),
        "Starting server"
    );
    axum::Server::bind(&listen_socket)
        .serve(router.into_make_service())
        .await
        .expect("Failed to start service");
}

pub async fn add_user(
    store_path: PathBuf,
    username: String,
    password: String,
    recipe_dir_path: Option<PathBuf>,
) {
    let app_store = storage::SqliteStore::new(store_path)
        .await
        .expect("Unable to create app_store");
    let user_creds = storage::UserCreds {
        id: storage::UserId(username.clone()),
        pass: secrecy::Secret::from(password),
    };
    app_store
        .store_user_creds(user_creds)
        .await
        .expect("Failed to store user creds");
    if let Some(path) = recipe_dir_path {
        let store = storage::file_store::AsyncFileStore::new(path);
        if let Some(recipes) = store
            .get_recipes()
            .await
            .expect("Unable to get recipes to load for user")
        {
            app_store
                .store_recipes_for_user(&username, &recipes)
                .await
                .expect("Failed to load user recipes");
        }
        if let Some(categories) = store
            .get_categories()
            .await
            .expect("Unable to get categories to fetch for user")
        {
            app_store
                .store_categories_for_user(&username, &categories)
                .await
                .expect("Failed to load user categories");
        }
        // TODO(jwall): Load all the recipes into our sqlite database
    }
}
