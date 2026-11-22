use crate::api::Result;
use theseus::prelude::*;

macro_rules! impl_cache_methods {
    ($(($variant:ident, $type:ty)),*) => {
        $(
            paste::paste! {
                #[tauri::command]
                pub async fn [<get_ $variant:snake>](id: &str, cache_behaviour: Option<CacheBehaviour>) -> Result<Option<$type>>
                {
                    Ok(theseus::cache::[<get_ $variant:snake>](id, cache_behaviour).await?)
                }

                #[tauri::command]
                pub async fn [<get_ $variant:snake _many>](
                    ids: Vec<String>,
                    cache_behaviour: Option<CacheBehaviour>,
                ) -> Result<Vec<$type>>
                {
                    let ids = ids.iter().map(|x| &**x).collect::<Vec<&str>>();
                    let entries =
                        theseus::cache::[<get_ $variant:snake _many>](&*ids, cache_behaviour).await?;

                    Ok(entries)
                }
            }
        )*
    }
}

impl_cache_methods!(
    (Project, Project),
    (ProjectV3, ProjectV3),
    (Version, Version),
    (User, User),
    (Team, Vec<TeamMember>),
    (Organization, Organization),
    (SearchResults, SearchResults),
    (SearchResultsV3, SearchResultsV3)
);

pub fn init<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> {
    tauri::plugin::Builder::new("cache")
        .invoke_handler(crate::commands::cache_handler!())
        .build()
}

#[tauri::command]
pub async fn purge_cache_types(cache_types: Vec<CacheValueType>) -> Result<()> {
    Ok(theseus::cache::purge_cache_types(&cache_types).await?)
}

#[tauri::command]
pub async fn get_project_versions(
    project_id: &str,
    cache_behaviour: Option<CacheBehaviour>,
) -> Result<Option<Vec<Version>>> {
    Ok(
        theseus::cache::get_project_versions(project_id, cache_behaviour)
            .await?,
    )
}
