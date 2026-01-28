use sqlx::postgres::PgPoolOptions;
use std::env;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let db = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await?;

    println!("Connected to database!");

    // Read themes from file
    let themes_content = std::fs::read_to_string("themes.txt")
        .expect("Failed to read themes.txt - make sure it exists!");

    let mut count = 0;
    let mut skipped = 0;

    for line in themes_content.lines() {
        let theme = line.trim();
        if theme.is_empty() || theme.starts_with('#') {
            continue;
        }

        // Check if theme already exists
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM themes WHERE content = $1)")
                .bind(theme)
                .fetch_one(&db)
                .await?;

        if exists {
            println!("⊘ Skipped (duplicate): {}", theme);
            skipped += 1;
            continue;
        }

        sqlx::query("INSERT INTO themes (content) VALUES ($1)")
            .bind(theme)
            .execute(&db)
            .await?;

        count += 1;
        println!("✓ Loaded: {}", theme);
    }

    println!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✓ Successfully loaded {} new themes!", count);
    if skipped > 0 {
        println!("⊘ Skipped {} duplicate themes", skipped);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");

    Ok(())
}
