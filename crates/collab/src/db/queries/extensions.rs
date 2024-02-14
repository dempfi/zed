use super::*;

impl Database {
    pub async fn get_extensions(
        &self,
        filter: Option<&str>,
        limit: u32,
    ) -> Result<Vec<(extension::Model, extension_version::Model)>> {
        self.transaction(|tx| async move {
            let extensions = if let Some(filter) = filter {
                // We need to upgrade `sea-orm` to get access to `ILIKE` in the query builder.
                // https://www.sea-ql.org/blog/2022-12-30-whats-new-in-seaquery-0.28.0/#api-additions
                let query = "
                    SELECT extensions.*
                    FROM extensions
                    WHERE name ILIKE $1
                    ORDER BY extensions.total_download_count DESC
                    LIMIT $2
                ";

                let fuzzy_name_filter = Self::fuzzy_like_string(filter);

                extension::Entity::find()
                    .from_raw_sql(Statement::from_sql_and_values(
                        self.pool.get_database_backend(),
                        query,
                        vec![fuzzy_name_filter.into(), limit.into()],
                    ))
                    .all(&*tx)
                    .await?
            } else {
                extension::Entity::find()
                    .order_by_desc(extension::Column::TotalDownloadCount)
                    .all(&*tx)
                    .await?
            };

            let latest_version_ids = extensions
                .iter()
                .filter_map(|extension| extension.latest_version)
                .collect::<Vec<_>>();

            let extension_versions = extension_version::Entity::find()
                .filter(extension_version::Column::Id.is_in(latest_version_ids))
                .all(&*tx)
                .await?;

            Ok(extensions
                .into_iter()
                .map(|extension| {
                    let latest_version = extension_versions
                        .iter()
                        .find(|version| Some(version.id) == extension.latest_version)
                        .unwrap();

                    (extension, latest_version.clone())
                })
                .collect())
        })
        .await
    }
}
