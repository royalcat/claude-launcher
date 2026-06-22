use crate::config::{get_all_credentials, mask_secret};
use crate::error::AppError;
use crate::providers::get_provider;

pub fn list_credentials() -> Result<(), AppError> {
    let all = get_all_credentials()?;
    let mut slugs: Vec<String> = all.keys().cloned().collect();
    slugs.sort();

    if slugs.is_empty() {
        eprintln!("\n  No credentials configured yet.\n");
        return Ok(());
    }

    println!();
    for slug in &slugs {
        let c = &all[slug];
        let provider_name = get_provider(&c.provider).map(|p| p.name).unwrap_or(c.provider.as_str());
        let token = c
            .env
            .get("ANTHROPIC_AUTH_TOKEN")
            .map(|t| mask_secret(t))
            .unwrap_or_else(|| "not set".to_string());
        let url = c.env.get("ANTHROPIC_BASE_URL").cloned().unwrap_or_else(|| "not set".to_string());

        println!("  \x1b[1m{}\x1b[0m \x1b[2m[{}]\x1b[0m", c.name, slug);
        println!("    Provider: \x1b[36m{}\x1b[0m", provider_name);
        println!("    Base URL: {}", url);
        println!("    API Key:  {}", token);
        if let Some(model) = c.env.get("ANTHROPIC_DEFAULT_SONNET_MODEL") {
            println!("    Sonnet:   {}", model);
        }
        println!();
    }

    Ok(())
}
