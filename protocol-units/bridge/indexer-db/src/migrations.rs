use diesel::pg::PgConnection;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

// Embed migrations from the migrations directory
pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

pub fn run_migrations(conn: &mut PgConnection) {
	conn.run_pending_migrations(MIGRATIONS).expect("Failed to run migrations");
}
