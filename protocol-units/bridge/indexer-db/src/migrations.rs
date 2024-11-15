use diesel::pg::PgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

// Embed migrations from the migrations directory
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations(conn: &mut PgConnection) -> Result<(), anyhow::Error> {
	conn.run_pending_migrations(MIGRATIONS)
		.map_err(|e| anyhow::anyhow!("Failed to run migrations for bridge indexer db: {}", e))?;
	Ok(())
}
