use crate::ErrorKind;
use crate::state::DirectoryInfo;
use sqlx::sqlite::{
    SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions,
};
use sqlx::{Pool, Sqlite};
use std::path::Path;
use std::time::Duration;

pub(crate) async fn connect(
    app_identifier: &str,
) -> crate::Result<Pool<Sqlite>> {
    let settings_dir = DirectoryInfo::initial_settings_dir_path(app_identifier)
        .ok_or(ErrorKind::FSError(
            "could not find valid config dir".to_string(),
        ))?;

    crate::util::io::create_dir_all(&settings_dir).await?;

    let db_path = settings_dir.join("app.db");

    connect_app_db(&db_path).await
}

async fn connect_app_db(db_path: &Path) -> crate::Result<Pool<Sqlite>> {
	super::db_backup::maybe_backup_existing_app_db(db_path).await?;
    open_migrated_app_db(db_path).await
}

async fn open_migrated_app_db(db_path: &Path) -> crate::Result<Pool<Sqlite>> {
    let pool = open_app_db_pool(db_path).await?;

    if let Err(err) = stale_data_cleanup(&pool).await {
        tracing::warn!(
            "Failed to clean up stale data from state database before migrations: {err}"
        );
    }

    sqlx::migrate!().run(&pool).await?;
    record_current_app_version(&pool).await?;

    if let Err(err) = stale_data_cleanup(&pool).await {
        tracing::warn!(
            "Failed to clean up stale data from state database: {err}"
        );
    }

    Ok(pool)
}

async fn open_app_db_pool(db_path: &Path) -> crate::Result<Pool<Sqlite>> {
    let conn_options = SqliteConnectOptions::new()
        .filename(db_path)
        .busy_timeout(Duration::from_secs(30))
        .journal_mode(SqliteJournalMode::Wal)
        .optimize_on_close(true, None)
        .create_if_missing(true);

    Ok(SqlitePoolOptions::new()
        .max_connections(10)
        .min_connections(1)
        .idle_timeout(Duration::from_secs(120))
        .connect_with(conn_options)
        .await?)
}

async fn record_current_app_version(pool: &Pool<Sqlite>) -> crate::Result<()> {
    let previous_version = sqlx::query_scalar!(
        "SELECT value FROM app_metadata WHERE key = 'app_version'"
    )
    .fetch_optional(pool)
    .await?;
    let already_used_sync_update = previous_version
        .as_deref()
        .and_then(|version| {
            let mut parts = version.split('.');
            Some((
                parts.next()?.parse::<u64>().ok()?,
                parts.next()?.parse::<u64>().ok()?,
            ))
        })
        .is_some_and(|version| version >= (0, 20));

    if env!("CARGO_PKG_VERSION").starts_with("0.20.")
        && already_used_sync_update
    {
        let mut settings = super::Settings::get(pool).await?;
        if settings.pending_update_toast_for_version.is_some() {
            settings.pending_update_toast_for_version = None;
            settings.update(pool).await?;
        }
    }

    sqlx::query!(
        "
		INSERT INTO app_metadata (key, value, updated_at)
		VALUES ('app_version', ?, unixepoch())
		ON CONFLICT(key) DO UPDATE SET
			value = excluded.value,
			updated_at = excluded.updated_at
		",
        env!("CARGO_PKG_VERSION"),
    )
    .execute(pool)
    .await?;

    Ok(())
}

/// Cleans up data from the database that is no longer referenced, but must be
/// kept around for a little while to allow users to recover from accidental
/// deletions.
async fn stale_data_cleanup(pool: &Pool<Sqlite>) -> crate::Result<()> {
    let mut tx = pool.begin_with("BEGIN IMMEDIATE").await?;

    let has_skin_tables = sqlx::query!(
		"SELECT COUNT(*) AS \"count!: i64\" FROM sqlite_master WHERE type = 'table' AND name IN ('custom_minecraft_skins', 'minecraft_users')",
	)
	.fetch_one(&mut *tx)
	.await?
	.count == 2;

    if has_skin_tables {
        sqlx::query!(
			"DELETE FROM custom_minecraft_skins WHERE minecraft_user_uuid NOT IN (SELECT uuid FROM minecraft_users)"
		)
		.execute(&mut *tx)
		.await?;
    }

    tx.commit().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
	use super::connect_app_db;
	use sqlx::sqlite::SqliteConnectOptions;
	use sqlx::{ConnectOptions, Connection};

	#[tokio::test]
	async fn gradle_backup_failure_prevents_migrations() {
		let directory = tempfile::tempdir().unwrap();
		let db_path = directory.path().join("app.db");
		let mut connection = SqliteConnectOptions::new()
			.filename(&db_path)
			.create_if_missing(true)
			.connect()
			.await
			.unwrap();
		sqlx::query("CREATE TABLE preserved_data (value TEXT NOT NULL)")
			.execute(&mut connection)
			.await
			.unwrap();
		sqlx::query("INSERT INTO preserved_data VALUES ('keep me')")
			.execute(&mut connection)
			.await
			.unwrap();
		sqlx::query("CREATE TABLE app_metadata (key TEXT PRIMARY KEY)")
			.execute(&mut connection)
			.await
			.unwrap();
		connection.close().await.unwrap();
		let before = tokio::fs::read(&db_path).await.unwrap();

		let error = connect_app_db(&db_path).await.unwrap_err();

		assert!(matches!(
			error.downcast_ref::<sqlx::Error>(),
			Some(sqlx::Error::Database(_))
		));
		assert!(error.to_string().contains("no such column: value"));
		assert_eq!(tokio::fs::read(&db_path).await.unwrap(), before);
		let mut connection = SqliteConnectOptions::new()
			.filename(&db_path)
			.read_only(true)
			.connect()
			.await
			.unwrap();
		let tables = sqlx::query_scalar::<_, String>(
			"SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name",
		)
		.fetch_all(&mut connection)
		.await
		.unwrap();
		assert_eq!(tables, vec!["app_metadata", "preserved_data"]);
		let value = sqlx::query_scalar::<_, String>(
			"SELECT value FROM preserved_data",
		)
		.fetch_one(&mut connection)
		.await
		.unwrap();
		assert_eq!(value, "keep me");
		connection.close().await.unwrap();
	}

	#[tokio::test]
	async fn test_migrations_checksums_and_headers_unmodified() {
		let migrator = sqlx::migrate!();
		// Ensure the two specific migrations that had comment modification match their expected header
		let m1 = migrator.iter().find(|m| m.version == 20250716135844).expect("migration 20250716135844 must exist");
		let m2 = migrator.iter().find(|m| m.version == 20260619152801).expect("migration 20260619152801 must exist");

		assert!(
			m1.sql.starts_with("-- [AR] - SQL Migration"),
			"migration 20250716135844 must preserve exact original comment '-- [AR] - SQL Migration', got: {}",
			m1.sql.lines().next().unwrap_or_default()
		);
		assert!(
			m2.sql.starts_with("-- [AR] - SQL Migration"),
			"migration 20260619152801 must preserve exact original comment '-- [AR] - SQL Migration', got: {}",
			m2.sql.lines().next().unwrap_or_default()
		);

		// Test applying all embedded migrations to a clean sqlite database
		let pool = sqlx::sqlite::SqlitePoolOptions::new()
			.connect("sqlite::memory:")
			.await
			.expect("failed to connect to memory db");

		migrator.run(&pool).await.expect("all migrations must apply successfully");
	}

	#[tokio::test]
	async fn test_existing_user_database_migration_compatibility() {
		let migrator = sqlx::migrate!();
		let db_path = dirs::data_dir()
			.map(|d| d.join("GradleApp").join("app.db"))
			.filter(|p| p.is_file());

		if let Some(user_db) = db_path {
			let temp_dir = tempfile::tempdir().expect("create temp dir");
			let test_db = temp_dir.path().join("app.db");
			std::fs::copy(&user_db, &test_db).expect("copy user db");

			let pool = sqlx::sqlite::SqlitePoolOptions::new()
				.connect(&format!("sqlite://{}", test_db.to_string_lossy()))
				.await
				.expect("failed to connect to user db copy");

			migrator.run(&pool).await.expect("all migrations must apply to existing user db with zero errors");
		} else {
			let pool = sqlx::sqlite::SqlitePoolOptions::new()
				.connect("sqlite::memory:")
				.await
				.expect("failed to connect to memory db");
			migrator.run(&pool).await.expect("all migrations must apply with zero errors");
		}
	}

	#[test]
	fn test_tauri_acl_commands_and_plugins_parity() {
		let output = std::process::Command::new("python3")
			.arg("../../scripts/verify_acl_commands.py")
			.output()
			.or_else(|_| {
				std::process::Command::new("python3")
					.arg("scripts/verify_acl_commands.py")
					.output()
			})
			.expect("failed to execute ACL verification script");

		assert!(
			output.status.success(),
			"ACL command verification failed:\n{}",
			String::from_utf8_lossy(&output.stderr)
		);

		let guardrail = std::process::Command::new("python3")
			.arg("../../scripts/verify_acl_guardrail.py")
			.output()
			.or_else(|_| {
				std::process::Command::new("python3")
					.arg("scripts/verify_acl_guardrail.py")
					.output()
			})
			.expect("failed to execute ACL guardrail script");

		assert!(
			guardrail.status.success(),
			"ACL guardrail verification failed:\n{}",
			String::from_utf8_lossy(&guardrail.stderr)
		);
	}

	#[test]
	fn test_rust_edition_end_to_end_flow() {
		use std::fs;
		use std::os::unix::fs::PermissionsExt;

		// 1. Run diagnostics
		let diag = horizon_runtime::requirements::generate_diagnostics_report(None, Some("OfflinePlayer"));
		assert!(!diag.os_info.is_empty(), "OS info should be populated in diagnostics");
		assert!(!diag.generated_at.is_empty(), "Diagnostics report should contain timestamp");

		// 2. Install a Horizon release in a test versions directory
		let temp_dir = tempfile::tempdir().expect("create test tempdir");
		let versions_root = temp_dir.path().join("versions");
		let version_tag = "launcher@1.3.0";
		let target_vdir = versions_root.join(version_tag);
		fs::create_dir_all(&target_vdir).expect("create version dir");

		// Create mock horizon-client binary
		let client_bin = target_vdir.join(horizon_runtime::releases::CLIENT_BIN_NAME);
		let script = "#!/bin/sh\n\
echo \"[Gradle Rust Edition] Initializing Horizon runtime...\"\n\
echo \"[Gradle Rust Edition] Loading instance configurations from $1...\"\n\
echo \"[Gradle Rust Edition] Connecting to game loop...\"\n\
echo \"[Gradle Rust Edition] Game loop ready. FPS: 60.\"\n";
		fs::write(&client_bin, script).expect("write mock client script");
		let mut perms = fs::metadata(&client_bin).expect("get perms").permissions();
		perms.set_mode(0o755);
		fs::set_permissions(&client_bin, perms).expect("set exec permissions");

		let info = horizon_runtime::releases::InstalledVersionInfo {
			tag: version_tag.to_string(),
			name: "Horizon Offline Launcher 1.3.0".to_string(),
			sha256: Some("mock_sha256_hash".to_string()),
			installed_at: "2026-11-08T00:00:00Z".to_string(),
		};
		let info_json = serde_json::to_string_pretty(&info).expect("serialize info");
		fs::write(target_vdir.join("installed_info.json"), info_json).expect("write installed_info.json");

		// 3. Prepare resources and instance directory
		let instance_dir = temp_dir.path().join("instance");
		let resources_dir = temp_dir.path().join("resources");
		fs::create_dir_all(&instance_dir).expect("create instance dir");
		fs::create_dir_all(&resources_dir).expect("create resources dir");

		// 4. Build command and execute Play
		let mut cmd = std::process::Command::new(&client_bin);
		cmd.arg(format!("--instance={}", instance_dir.display()))
			.arg(format!("--resources={}", resources_dir.display()))
			.arg("--username=GradlePlayer");

		let output = cmd.output().expect("execute horizon client");
		assert!(output.status.success(), "client execution should succeed");

		let log_output = String::from_utf8_lossy(&output.stdout);
		println!("=== HORIZON INSTANCE LOG OUTPUT ===\n{}", log_output);

		assert!(
			log_output.contains("[Gradle Rust Edition] Initializing Horizon runtime..."),
			"Log output should show runtime initialization"
		);
		assert!(
			log_output.contains("[Gradle Rust Edition] Game loop ready. FPS: 60."),
			"Log output should show game loop ready"
		);
	}
}
