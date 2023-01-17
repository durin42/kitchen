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
use std::collections::{BTreeMap, BTreeSet};

use base64;
use chrono::NaiveDate;
use reqwasm;
use serde_json::{from_str, to_string};
use sycamore::prelude::*;
use tracing::{debug, error, instrument};

use client_api::*;
use recipes::{IngredientKey, RecipeEntry};
use wasm_bindgen::JsValue;
use web_sys::Storage;

use crate::{app_state::AppState, js_lib};

#[derive(Debug)]
pub struct Error(String);

impl From<std::io::Error> for Error {
    fn from(item: std::io::Error) -> Self {
        Error(format!("{:?}", item))
    }
}

impl From<Error> for String {
    fn from(item: Error) -> Self {
        format!("{:?}", item)
    }
}

impl From<JsValue> for Error {
    fn from(item: JsValue) -> Self {
        Error(format!("{:?}", item))
    }
}

impl From<String> for Error {
    fn from(item: String) -> Self {
        Error(item)
    }
}

impl From<&'static str> for Error {
    fn from(item: &'static str) -> Self {
        Error(item.to_owned())
    }
}

impl From<std::string::FromUtf8Error> for Error {
    fn from(item: std::string::FromUtf8Error) -> Self {
        Error(format!("{:?}", item))
    }
}

impl From<reqwasm::Error> for Error {
    fn from(item: reqwasm::Error) -> Self {
        Error(format!("{:?}", item))
    }
}

fn recipe_key<S: std::fmt::Display>(id: S) -> String {
    format!("recipe:{}", id)
}

fn category_key<S: std::fmt::Display>(id: S) -> String {
    format!("category:{}", id)
}

fn token68(user: String, pass: String) -> String {
    base64::encode(format!("{}:{}", user, pass))
}

#[derive(Clone, Debug)]
pub struct LocalStore {
    store: Storage,
}

impl LocalStore {
    pub fn new() -> Self {
        Self {
            store: js_lib::get_storage(),
        }
    }

    /// Gets user data from local storage.
    pub fn get_user_data(&self) -> Option<UserData> {
        self.store
            .get("user_data")
            .map_or(None, |val| val.map(|val| from_str(&val).unwrap_or(None)))
            .flatten()
    }

    // Set's user data to local storage.
    pub fn set_user_data(&self, data: Option<&UserData>) {
        if let Some(data) = data {
            self.store
                .set(
                    "user_data",
                    &to_string(data).expect("Failed to desrialize user_data"),
                )
                .expect("Failed to set user_data");
        } else {
            self.store
                .delete("user_data")
                .expect("Failed to delete user_data");
        }
    }

    /// Gets categories from local storage.
    pub fn get_categories(&self) -> Option<Vec<(String, String)>> {
        let mut mappings = Vec::new();
        for k in self.get_category_keys() {
            if let Some(mut cat_map) = self
                .store
                .get(&k)
                .expect(&format!("Failed to get category key {}", k))
                .map(|v| {
                    from_str::<Vec<(String, String)>>(&v)
                        .expect(&format!("Failed to parse category key {}", k))
                })
            {
                mappings.extend(cat_map.drain(0..));
            }
        }
        if mappings.is_empty() {
            None
        } else {
            Some(mappings)
        }
    }

    /// Set the categories to the given string.
    pub fn set_categories(&self, mappings: Option<&Vec<(String, String)>>) {
        if let Some(mappings) = mappings {
            for (i, cat) in mappings.iter() {
                self.store
                    .set(
                        &category_key(i),
                        &to_string(&(i, cat)).expect("Failed to serialize category mapping"),
                    )
                    .expect("Failed to store category mapping");
            }
        }
    }

    fn get_storage_keys(&self) -> Vec<String> {
        let mut keys = Vec::new();
        for idx in 0..self.store.length().unwrap() {
            if let Some(k) = self.store.key(idx).expect("Failed to get storage key") {
                keys.push(k)
            }
        }
        keys
    }

    fn get_category_keys(&self) -> impl Iterator<Item = String> {
        self.get_storage_keys()
            .into_iter()
            .filter(|k| k.starts_with("category:"))
    }

    fn get_recipe_keys(&self) -> impl Iterator<Item = String> {
        self.get_storage_keys()
            .into_iter()
            .filter(|k| k.starts_with("recipe:"))
    }

    /// Gets all the recipes from local storage.
    pub fn get_recipes(&self) -> Option<Vec<RecipeEntry>> {
        let mut recipe_list = Vec::new();
        for recipe_key in self.get_recipe_keys() {
            if let Some(entry) = self
                .store
                .get(&recipe_key)
                .expect(&format!("Failed to get recipe: {}", recipe_key))
            {
                match from_str(&entry) {
                    Ok(entry) => {
                        recipe_list.push(entry);
                    }
                    Err(e) => {
                        error!(recipe_key, err = ?e, "Failed to parse recipe entry");
                    }
                }
            }
        }
        if recipe_list.is_empty() {
            return None;
        }
        Some(recipe_list)
    }

    pub fn get_recipe_entry(&self, id: &str) -> Option<RecipeEntry> {
        let key = recipe_key(id);
        self.store
            .get(&key)
            .expect(&format!("Failed to get recipe {}", key))
            .map(|entry| from_str(&entry).expect(&format!("Failed to get recipe {}", key)))
    }

    /// Sets the set of recipes to the entries passed in. Deletes any recipes not
    /// in the list.
    pub fn set_all_recipes(&self, entries: &Vec<RecipeEntry>) {
        for recipe_key in self.get_recipe_keys() {
            self.store
                .delete(&recipe_key)
                .expect(&format!("Failed to get recipe {}", recipe_key));
        }
        for entry in entries {
            self.set_recipe_entry(entry);
        }
    }

    /// Set recipe entry in local storage.
    pub fn set_recipe_entry(&self, entry: &RecipeEntry) {
        self.store
            .set(
                &recipe_key(entry.recipe_id()),
                &to_string(&entry).expect(&format!("Failed to get recipe {}", entry.recipe_id())),
            )
            .expect(&format!("Failed to store recipe {}", entry.recipe_id()))
    }

    /// Delete recipe entry from local storage.
    pub fn delete_recipe_entry(&self, recipe_id: &str) {
        self.store
            .delete(&recipe_key(recipe_id))
            .expect(&format!("Failed to delete recipe {}", recipe_id))
    }

    /// Save working plan to local storage.
    pub fn store_plan(&self, plan: &Vec<(String, i32)>) {
        self.store
            .set("plan", &to_string(&plan).expect("Failed to serialize plan"))
            .expect("Failed to store plan'");
    }

    pub fn get_plan(&self) -> Option<Vec<(String, i32)>> {
        if let Some(plan) = self.store.get("plan").expect("Failed to store plan") {
            Some(from_str(&plan).expect("Failed to deserialize plan"))
        } else {
            None
        }
    }

    pub fn set_plan_date(&self, date: &NaiveDate) {
        self.store
            .set(
                "plan:date",
                &to_string(&date).expect("Failed to serialize plan:date"),
            )
            .expect("Failed to store plan:date");
    }

    pub fn get_plan_date(&self) -> Option<NaiveDate> {
        if let Some(date) = self
            .store
            .get("plan:date")
            .expect("Failed to get plan date")
        {
            Some(from_str(&date).expect("Failed to deserialize plan_date"))
        } else {
            None
        }
    }

    pub fn get_inventory_data(
        &self,
    ) -> Option<(
        BTreeSet<IngredientKey>,
        BTreeMap<IngredientKey, String>,
        Vec<(String, String)>,
    )> {
        if let Some(inventory) = self
            .store
            .get("inventory")
            .expect("Failed to retrieve inventory data")
        {
            let (filtered, modified, extras): (
                BTreeSet<IngredientKey>,
                Vec<(IngredientKey, String)>,
                Vec<(String, String)>,
            ) = from_str(&inventory).expect("Failed to deserialize inventory");
            return Some((filtered, BTreeMap::from_iter(modified), extras));
        }
        return None;
    }

    pub fn set_inventory_data(
        &self,
        inventory: (
            &BTreeSet<IngredientKey>,
            &BTreeMap<IngredientKey, String>,
            &Vec<(String, String)>,
        ),
    ) {
        let filtered = inventory.0;
        let modified_amts = inventory
            .1
            .iter()
            .map(|(k, amt)| (k.clone(), amt.clone()))
            .collect::<Vec<(IngredientKey, String)>>();
        let extras = inventory.2;
        let inventory_data = (filtered, &modified_amts, extras);
        self.store
            .set(
                "inventory",
                &to_string(&inventory_data).expect(&format!(
                    "Failed to serialize inventory {:?}",
                    inventory_data
                )),
            )
            .expect("Failed to set inventory");
    }

    pub fn set_staples(&self, content: &String) {
        self.store
            .set("staples", content)
            .expect("Failed to set staples in local store");
    }

    pub fn get_staples(&self) -> Option<String> {
        self.store
            .get("staples")
            .expect("Failed to retreive staples from local store")
    }
}

#[derive(Clone, Debug)]
pub struct HttpStore {
    root: String,
    local_store: LocalStore,
}

impl HttpStore {
    pub fn new(root: String) -> Self {
        Self {
            root,
            local_store: LocalStore::new(),
        }
    }

    pub fn v2_path(&self) -> String {
        let mut path = self.root.clone();
        path.push_str("/v2");
        path
    }

    pub fn provide_context<S: Into<String>>(cx: Scope, root: S) {
        provide_context(cx, std::rc::Rc::new(Self::new(root.into())));
    }

    pub fn get_from_context(cx: Scope) -> std::rc::Rc<Self> {
        use_context::<std::rc::Rc<Self>>(cx).clone()
    }

    // NOTE(jwall): We do **not** want to record the password in our logs.
    #[instrument(skip_all, fields(?self, user))]
    pub async fn authenticate(&self, user: String, pass: String) -> Option<UserData> {
        debug!("attempting login request against api.");
        let mut path = self.v2_path();
        path.push_str("/auth");
        let result = reqwasm::http::Request::get(&path)
            .header(
                "Authorization",
                format!("Basic {}", token68(user, pass)).as_str(),
            )
            .send()
            .await;
        if let Ok(resp) = &result {
            if resp.status() == 200 {
                let user_data = resp
                    .json::<AccountResponse>()
                    .await
                    .expect("Unparseable authentication response")
                    .as_success();
                return user_data;
            }
            error!(status = resp.status(), "Login was unsuccessful")
        } else {
            error!(err=?result.unwrap_err(), "Failed to send auth request");
        }
        return None;
    }

    #[instrument]
    pub async fn fetch_user_data(&self) -> Option<UserData> {
        debug!("Retrieving User Account data");
        let mut path = self.v2_path();
        path.push_str("/account");
        let result = reqwasm::http::Request::get(&path).send().await;
        if let Ok(resp) = &result {
            if resp.status() == 200 {
                let user_data = resp
                    .json::<AccountResponse>()
                    .await
                    .expect("Unparseable authentication response")
                    .as_success();
                return user_data;
            }
            error!(status = resp.status(), "Login was unsuccessful")
        } else {
            error!(err=?result.unwrap_err(), "Failed to send auth request");
        }
        return None;
    }

    //#[instrument]
    pub async fn fetch_categories(&self) -> Result<Option<Vec<(String, String)>>, Error> {
        let mut path = self.v2_path();
        path.push_str("/category_map");
        let resp = match reqwasm::http::Request::get(&path).send().await {
            Ok(resp) => resp,
            Err(reqwasm::Error::JsError(err)) => {
                error!(path, ?err, "Error hitting api");
                return Ok(self.local_store.get_categories());
            }
            Err(err) => {
                return Err(err)?;
            }
        };
        if resp.status() == 404 {
            debug!("Categories returned 404");
            Ok(None)
        } else if resp.status() != 200 {
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back!");
            let resp = resp
                .json::<CategoryMappingResponse>()
                .await?
                .as_success()
                .unwrap();
            Ok(Some(resp))
        }
    }

    #[instrument]
    pub async fn fetch_recipes(&self) -> Result<Option<Vec<RecipeEntry>>, Error> {
        let mut path = self.v2_path();
        path.push_str("/recipes");
        let resp = match reqwasm::http::Request::get(&path).send().await {
            Ok(resp) => resp,
            Err(reqwasm::Error::JsError(err)) => {
                error!(path, ?err, "Error hitting api");
                return Ok(self.local_store.get_recipes());
            }
            Err(err) => {
                return Err(err)?;
            }
        };
        if resp.status() != 200 {
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back!");
            let entries = resp
                .json::<RecipeEntryResponse>()
                .await
                .map_err(|e| format!("{}", e))?
                .as_success();
            Ok(entries)
        }
    }

    pub async fn fetch_recipe_text<S: AsRef<str> + std::fmt::Display>(
        &self,
        id: S,
    ) -> Result<Option<RecipeEntry>, Error> {
        let mut path = self.v2_path();
        path.push_str("/recipe/");
        path.push_str(id.as_ref());
        let resp = match reqwasm::http::Request::get(&path).send().await {
            Ok(resp) => resp,
            Err(reqwasm::Error::JsError(err)) => {
                error!(path, ?err, "Error hitting api");
                return Ok(self.local_store.get_recipe_entry(id.as_ref()));
            }
            Err(err) => {
                return Err(err)?;
            }
        };
        if resp.status() != 200 {
            Err(format!("Status: {}", resp.status()).into())
        } else if resp.status() == 404 {
            debug!("Recipe doesn't exist");
            Ok(None)
        } else {
            debug!("We got a valid response back!");
            let entry = resp
                .json::<Response<Option<RecipeEntry>>>()
                .await
                .map_err(|e| format!("{}", e))?
                .as_success()
                .unwrap();
            if let Some(ref entry) = entry {
                self.local_store.set_recipe_entry(entry);
            }
            Ok(entry)
        }
    }

    #[instrument]
    pub async fn delete_recipe<S>(&self, recipe: S) -> Result<(), Error>
    where
        S: AsRef<str> + std::fmt::Debug,
    {
        let mut path = self.v2_path();
        path.push_str("/recipe");
        path.push_str(&format!("/{}", recipe.as_ref()));
        let resp = reqwasm::http::Request::delete(&path).send().await?;
        if resp.status() != 200 {
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back!");
            Ok(())
        }
    }

    #[instrument(skip(recipes), fields(count=recipes.len()))]
    pub async fn store_recipes(&self, recipes: Vec<RecipeEntry>) -> Result<(), Error> {
        let mut path = self.v2_path();
        path.push_str("/recipes");
        for r in recipes.iter() {
            if r.recipe_id().is_empty() {
                return Err("Recipe Ids can not be empty".into());
            }
        }
        let serialized = to_string(&recipes).expect("Unable to serialize recipe entries");
        let resp = reqwasm::http::Request::post(&path)
            .body(&serialized)
            .header("content-type", "application/json")
            .send()
            .await?;
        if resp.status() != 200 {
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back!");
            Ok(())
        }
    }

    #[instrument(skip(categories))]
    pub async fn store_categories(&self, categories: &Vec<(String, String)>) -> Result<(), Error> {
        let mut path = self.v2_path();
        path.push_str("/category_map");
        let resp = reqwasm::http::Request::post(&path)
            .body(to_string(&categories).expect("Unable to encode categories as json"))
            .header("content-type", "application/json")
            .send()
            .await?;
        if resp.status() != 200 {
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back!");
            Ok(())
        }
    }

    #[instrument(skip_all)]
    pub async fn store_app_state(&self, state: AppState) -> Result<(), Error> {
        let mut plan = Vec::new();
        for (key, count) in state.recipe_counts.iter() {
            plan.push((key.clone(), *count as i32));
        }
        debug!("Saving plan data");
        self.store_plan(plan).await?;
        debug!("Saving inventory data");
        self.store_inventory_data(
            state.filtered_ingredients,
            state.modified_amts,
            state
                .extras
                .iter()
                .cloned()
                .collect::<Vec<(String, String)>>(),
        )
        .await
    }

    pub async fn store_plan(&self, plan: Vec<(String, i32)>) -> Result<(), Error> {
        let mut path = self.v2_path();
        path.push_str("/plan");
        let resp = reqwasm::http::Request::post(&path)
            .body(to_string(&plan).expect("Unable to encode plan as json"))
            .header("content-type", "application/json")
            .send()
            .await?;
        if resp.status() != 200 {
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back!");
            Ok(())
        }
    }

    pub async fn fetch_plan_dates(&self) -> Result<Option<Vec<NaiveDate>>, Error> {
        let mut path = self.v2_path();
        path.push_str("/plan");
        path.push_str("/all");
        let resp = reqwasm::http::Request::get(&path).send().await?;
        if resp.status() != 200 {
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back");
            let plan = resp
                .json::<Response<Vec<NaiveDate>>>()
                .await
                .map_err(|e| format!("{}", e))?
                .as_success();
            Ok(plan)
        }
    }

    pub async fn fetch_plan_for_date(
        &self,
        date: &NaiveDate,
    ) -> Result<Option<Vec<(String, i32)>>, Error> {
        let mut path = self.v2_path();
        path.push_str("/plan");
        path.push_str("/at");
        path.push_str(&format!("/{}", date));
        let resp = reqwasm::http::Request::get(&path).send().await?;
        if resp.status() != 200 {
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back");
            let plan = resp
                .json::<PlanDataResponse>()
                .await
                .map_err(|e| format!("{}", e))?
                .as_success();
            Ok(plan)
        }
    }

    pub async fn fetch_plan(&self) -> Result<Option<Vec<(String, i32)>>, Error> {
        let mut path = self.v2_path();
        path.push_str("/plan");
        let resp = reqwasm::http::Request::get(&path).send().await?;
        if resp.status() != 200 {
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back");
            let plan = resp
                .json::<PlanDataResponse>()
                .await
                .map_err(|e| format!("{}", e))?
                .as_success();
            Ok(plan)
        }
    }

    pub async fn fetch_inventory_for_date(
        &self,
        date: &NaiveDate,
    ) -> Result<
        (
            BTreeSet<IngredientKey>,
            BTreeMap<IngredientKey, String>,
            Vec<(String, String)>,
        ),
        Error,
    > {
        let mut path = self.v2_path();
        path.push_str("/inventory");
        path.push_str("/at");
        path.push_str(&format!("/{}", date));
        let resp = reqwasm::http::Request::get(&path).send().await?;
        if resp.status() != 200 {
            let err = Err(format!("Status: {}", resp.status()).into());
            Ok(match self.local_store.get_inventory_data() {
                Some(val) => val,
                None => return err,
            })
        } else {
            debug!("We got a valid response back");
            let InventoryData {
                filtered_ingredients,
                modified_amts,
                extra_items,
            } = resp
                .json::<InventoryResponse>()
                .await
                .map_err(|e| format!("{}", e))?
                .as_success()
                .unwrap();
            Ok((
                filtered_ingredients.into_iter().collect(),
                modified_amts.into_iter().collect(),
                extra_items,
            ))
        }
    }

    pub async fn fetch_inventory_data(
        &self,
    ) -> Result<
        (
            BTreeSet<IngredientKey>,
            BTreeMap<IngredientKey, String>,
            Vec<(String, String)>,
        ),
        Error,
    > {
        let mut path = self.v2_path();
        path.push_str("/inventory");
        let resp = reqwasm::http::Request::get(&path).send().await?;
        if resp.status() != 200 {
            let err = Err(format!("Status: {}", resp.status()).into());
            Ok(match self.local_store.get_inventory_data() {
                Some(val) => val,
                None => return err,
            })
        } else {
            debug!("We got a valid response back");
            let InventoryData {
                filtered_ingredients,
                modified_amts,
                extra_items,
            } = resp
                .json::<InventoryResponse>()
                .await
                .map_err(|e| format!("{}", e))?
                .as_success()
                .unwrap();
            Ok((
                filtered_ingredients.into_iter().collect(),
                modified_amts.into_iter().collect(),
                extra_items,
            ))
        }
    }

    #[instrument]
    pub async fn store_inventory_data(
        &self,
        filtered_ingredients: BTreeSet<IngredientKey>,
        modified_amts: BTreeMap<IngredientKey, String>,
        extra_items: Vec<(String, String)>,
    ) -> Result<(), Error> {
        let mut path = self.v2_path();
        path.push_str("/inventory");
        let filtered_ingredients: Vec<IngredientKey> = filtered_ingredients.into_iter().collect();
        let modified_amts: Vec<(IngredientKey, String)> = modified_amts.into_iter().collect();
        debug!("Storing inventory data in cache");
        let serialized_inventory = to_string(&(filtered_ingredients, modified_amts, extra_items))
            .expect("Unable to encode plan as json");
        debug!("Storing inventory data via API");
        let resp = reqwasm::http::Request::post(&path)
            .body(&serialized_inventory)
            .header("content-type", "application/json")
            .send()
            .await?;
        if resp.status() != 200 {
            debug!("Invalid response back");
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back!");
            Ok(())
        }
    }

    pub async fn fetch_staples(&self) -> Result<Option<String>, Error> {
        let mut path = self.v2_path();
        path.push_str("/staples");
        let resp = reqwasm::http::Request::get(&path).send().await?;
        if resp.status() != 200 {
            debug!("Invalid response back");
            Err(format!("Status: {}", resp.status()).into())
        } else {
            Ok(resp
                .json::<Response<Option<String>>>()
                .await
                .expect("Failed to parse staples json")
                .as_success()
                .unwrap())
        }
    }

    pub async fn store_staples<S: AsRef<str>>(&self, content: S) -> Result<(), Error> {
        let mut path = self.v2_path();
        path.push_str("/staples");
        let serialized_staples: String =
            to_string(content.as_ref()).expect("Failed to serialize staples to json");

        let resp = reqwasm::http::Request::post(&path)
            .body(&serialized_staples)
            .header("content-type", "application/json")
            .send()
            .await?;
        if resp.status() != 200 {
            debug!("Invalid response back");
            Err(format!("Status: {}", resp.status()).into())
        } else {
            debug!("We got a valid response back!");
            Ok(())
        }
    }
}
