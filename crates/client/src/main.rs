use axum::{Router, extract::Query, response::Html, routing::get};
use colored::*;
use serde::{Deserialize, Serialize};
use std::env;
use std::io::{self, Write};
use std::sync::{Arc, Mutex};

const BACKEND_URL: &str = "http://localhost:3000";
const CALLBACK_PORT: u16 = 8080;

// ===== Models =====

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Theme {
    id: i64,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ThemeResponse {
    theme: Option<Theme>,
    total: i64,
    seen: i64,
}

#[derive(Debug, Serialize)]
struct VoteRequest {
    theme_id: i64,
    vote_type: String,
}

#[derive(Debug, Deserialize)]
struct CallbackParams {
    access_token: Option<String>,
    error: Option<String>,
}

// ===== Main =====

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let supabase_url = env::var("SUPABASE_URL").expect("DATABASE_URL must be set");
    println!("{}", "=".repeat(60).bright_cyan());
    println!(
        "{}",
        "    🎮 BEVY JAM THEME VOTING 🎮".bright_yellow().bold()
    );
    println!("{}", "=".repeat(60).bright_cyan());
    println!();

    // Get auth token
    let token = match authenticate(supabase_url).await {
        Ok(t) => t,
        Err(e) => {
            eprintln!("{} {}", "❌ Authentication failed:".red().bold(), e);
            return Ok(());
        }
    };

    println!("{}", "✅ Authentication successful!".green().bold());
    println!();

    // Start voting loop
    voting_loop(&token).await?;

    Ok(())
}

// ===== Authentication =====

async fn authenticate(supabase_url: String) -> anyhow::Result<String> {
    println!("Starting authentication...");
    println!();

    // Token storage shared between server and main thread
    let token_store: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
    let token_store_clone = token_store.clone();

    // Build OAuth callback server
    let app = Router::new().route(
        "/callback",
        get(move |query: Query<CallbackParams>| callback_handler(query, token_store_clone.clone())),
    );

    // Start server in background
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", CALLBACK_PORT)).await?;
    println!(
        "{}",
        format!("🔓 Local callback server started on port {}", CALLBACK_PORT).cyan()
    );

    let server_handle = tokio::spawn(async move { axum::serve(listener, app).await });

    // Build auth URL
    let auth_url = format!(
        "{}/auth/v1/authorize?provider=discord&redirect_to=http://localhost:{}/callback",
        supabase_url, CALLBACK_PORT
    );

    println!();
    println!("{}", "Opening browser for Discord login...".yellow());
    println!();

    // Open browser
    if let Err(e) = webbrowser::open(&auth_url) {
        eprintln!(
            "{} {}",
            "⚠️  Could not open browser automatically:".yellow(),
            e
        );
        println!();
        println!("{}", "Please open this URL manually:".bright_white().bold());
        println!("{}", auth_url.bright_blue().underline());
        println!();
    }

    // Wait for token (with timeout)
    let timeout = tokio::time::Duration::from_secs(120);
    let start = tokio::time::Instant::now();

    loop {
        tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

        if let Some(token) = token_store.lock().unwrap().clone() {
            // Got token! Abort server
            server_handle.abort();
            return Ok(token);
        }

        if start.elapsed() > timeout {
            server_handle.abort();
            anyhow::bail!("Authentication timeout (2 minutes)");
        }
    }
}

async fn callback_handler(
    Query(params): Query<CallbackParams>,
    token_store: Arc<Mutex<Option<String>>>,
) -> Html<String> {
    let s = token_store.lock();
    let Ok(mut store) = s else {
        return Html("failed".to_string());
    };
    *store = params.access_token;
    if let Some(error) = params.error {
        return Html(format!(
            r#"
            <!DOCTYPE html>
            <html>
            <head>
                <title>Authentication Error</title>
                <style>
                    body {{
                        font-family: Arial, sans-serif;
                        display: flex;
                        justify-content: center;
                        align-items: center;
                        height: 100vh;
                        margin: 0;
                        background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                    }}
                    .container {{
                        background: white;
                        padding: 3rem;
                        border-radius: 1rem;
                        box-shadow: 0 10px 40px rgba(0,0,0,0.3);
                        text-align: center;
                        max-width: 500px;
                    }}
                    .error {{ color: #e53e3e; font-size: 1.5rem; margin: 1rem 0; }}
                    .message {{ color: #666; }}
                </style>
            </head>
            <body>
                <div class="container">
                    <h1 class="error">❌ Authentication Failed</h1>
                    <p class="message">{}</p>
                    <p>You can close this window and try again.</p>
                </div>
            </body>
            </html>
            "#,
            error
        ));
    }

    // Extract token from hash fragment (it comes after #, not in query params)
    // JavaScript will handle this
    Html(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Authentication Success</title>
            <style>
                body {
                    font-family: Arial, sans-serif;
                    display: flex;
                    justify-content: center;
                    align-items: center;
                    height: 100vh;
                    margin: 0;
                    background: linear-gradient(135deg, #667eea 0%, #764ba2 100%);
                }
                .container {
                    background: white;
                    padding: 3rem;
                    border-radius: 1rem;
                    box-shadow: 0 10px 40px rgba(0,0,0,0.3);
                    text-align: center;
                    max-width: 500px;
                }
                .success { color: #38a169; font-size: 2rem; }
                .loading { color: #666; margin-top: 1rem; }
                @keyframes spin {
                    to { transform: rotate(360deg); }
                }
                .spinner {
                    display: inline-block;
                    width: 20px;
                    height: 20px;
                    border: 3px solid #ddd;
                    border-top-color: #667eea;
                    border-radius: 50%;
                    animation: spin 1s linear infinite;
                    margin-left: 10px;
                }
            </style>
        </head>
        <body>
            <div class="container">
                <h1 class="success">✅ Authentication Successful!</h1>
                <p class="loading">Redirecting back to app<span class="spinner"></span></p>
                <p style="color: #999; font-size: 0.9rem; margin-top: 2rem;">
                    You can close this window if not redirected automatically.
                </p>
            </div>
            <script>
                // Extract token from URL hash
                const hash = window.location.hash.substring(1);
                const params = new URLSearchParams(hash);
                const token = params.get('access_token');
                
                if (token) {
                    // Send token to our callback endpoint
                    fetch('/callback?access_token=' + encodeURIComponent(token))
                        .then(() => {
                            setTimeout(() => window.close(), 1000);
                        });
                } else {
                    document.querySelector('.container').innerHTML = 
                        '<h1 style="color: #e53e3e;">❌ No token found</h1>' +
                        '<p>Please try logging in again.</p>';
                }
            </script>
        </body>
        </html>
        "#
        .to_string(),
    )
    .into()
}

// ===== Voting Loop =====

async fn voting_loop(token: &str) -> anyhow::Result<()> {
    loop {
        // Fetch next theme
        println!("Fetching next theme...");
        let response = fetch_next_theme(token).await?;

        if let Some(theme) = response.theme {
            println!("{}", "━".repeat(60).bright_black());
            println!();
            println!(
                "{} {}/{}",
                "Progress:".bright_black(),
                response.seen.to_string().bright_cyan(),
                response.total.to_string().bright_cyan()
            );
            println!();
            println!("{}", "THEME:".bright_yellow().bold());
            println!("{}", theme.content.bright_white().bold());
            println!();
            println!(
                "{}",
                "Vote: [Y]es  [N]o  [S]kip  [Q]uit  [R]esults".bright_black()
            );
            print!("{}", "> ".bright_green().bold());
            io::stdout().flush()?;

            // Get user input
            let mut input = String::new();
            io::stdin().read_line(&mut input)?;
            let choice = input.trim().to_lowercase();

            match choice.as_str() {
                "y" | "yes" => {
                    submit_vote(theme.id, "yes", token).await?;
                    println!("{}", "✓ Voted YES".green());
                }
                "n" | "no" => {
                    submit_vote(theme.id, "no", token).await?;
                    println!("{}", "✓ Voted NO".red());
                }
                "s" | "skip" => {
                    submit_vote(theme.id, "skip", token).await?;
                    println!("{}", "→ Skipped".yellow());
                }
                "q" | "quit" => {
                    println!();
                    println!("{}", "Thanks for voting! 👋".bright_cyan().bold());
                    return Ok(());
                }
                "r" | "results" => {
                    show_results().await?;
                    continue;
                }
                _ => {
                    println!("{}", "Invalid choice. Please try again.".red());
                    continue;
                }
            }
        } else {
            println!();
            println!("{}", "🎉 You've voted on all themes!".green().bold());
            println!();
            println!("View results? [Y/n]");
            print!("> ");
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().to_lowercase().starts_with('n') {
                show_results().await?;
            }

            break;
        }
    }

    Ok(())
}

// ===== API Calls =====

async fn fetch_next_theme(token: &str) -> anyhow::Result<ThemeResponse> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/themes/next", BACKEND_URL))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("API error ({}): {}", status, text);
    }

    Ok(response.json().await?)
}

async fn submit_vote(theme_id: i64, vote_type: &str, token: &str) -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let vote_req = VoteRequest {
        theme_id,
        vote_type: vote_type.to_string(),
    };

    let response = client
        .post(format!("{}/themes/vote", BACKEND_URL))
        .header("Authorization", format!("Bearer {}", token))
        .json(&vote_req)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let text = response.text().await?;
        anyhow::bail!("Vote failed ({}): {}", status, text);
    }

    Ok(())
}

async fn show_results() -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let response = client
        .get(format!("{}/admin/stats", BACKEND_URL))
        .send()
        .await?
        .json::<Vec<serde_json::Value>>()
        .await?;

    println!();
    println!("{}", "=".repeat(60).bright_cyan());
    println!("{}", "    📊 VOTING RESULTS".bright_yellow().bold());
    println!("{}", "=".repeat(60).bright_cyan());
    println!();

    for (i, theme) in response.iter().enumerate().take(10) {
        let content = theme["content"].as_str().unwrap_or("Unknown");
        let yes = theme["yes_votes"].as_i64().unwrap_or(0);
        let no = theme["no_votes"].as_i64().unwrap_or(0);
        let total = theme["total_votes"].as_i64().unwrap_or(0);

        println!(
            "{}. {} ({} votes: {} yes, {} no)",
            (i + 1).to_string().bright_cyan(),
            content.bright_white().bold(),
            total.to_string().yellow(),
            yes.to_string().green(),
            no.to_string().red()
        );
    }

    println!();
    Ok(())
}
