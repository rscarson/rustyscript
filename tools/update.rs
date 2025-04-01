use reqwest::{Client, header::USER_AGENT};
use serde::Deserialize;
use std::fs;
use toml_edit::DocumentMut;

static UPDATER_USER_AGENT: &str = "rustyscript/updater/0.1";

#[derive(Debug, Deserialize)]
struct Dependency {
    crate_id: String,
    req: String,
}

#[derive(Debug, Deserialize)]
struct DependenciesResponse {
    dependencies: Vec<Dependency>,
}

#[derive(Debug, Deserialize)]
struct CapacityBuilderMacrosVersion {
    num: String,
}

#[derive(Debug, Deserialize)]
struct CapacityBuilderMacrosResponse {
    versions: Vec<CapacityBuilderMacrosVersion>,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // rustyscript dependencies
    let path = "../Cargo.toml";
    let content = fs::read_to_string(path)?;
    let mut doc = content.parse::<DocumentMut>()?;

    let deno_version = match std::env::args().nth(1) {
        Some(version) => version,
        None => get_latest_deno_version().await?,
    };
    let deno_deps = get_remote_dependencies("deno", &deno_version).await?;
    compare_and_apply_deps(&mut doc, &deno_deps)?;

    let mut deno_runtime_version = deno_deps
        .iter()
        .find(|d| d.crate_id == "deno_runtime")
        .ok_or("deno_runtime not found")?
        .req
        .clone();
    if deno_runtime_version.starts_with('^') {
        deno_runtime_version = deno_runtime_version[1..].to_string();
    }
    let deno_runtime_deps = get_remote_dependencies("deno_runtime", &deno_runtime_version).await?;
    compare_and_apply_deps(&mut doc, &deno_runtime_deps)?;

    fs::write(path, doc.to_string())?;
    println!("Updated dependencies successfully!");
    Ok(())
}

async fn get_latest_deno_version() -> Result<String, Box<dyn std::error::Error>> {
    let url = "https://crates.io/api/v1/crates/deno?include=default_version";
    let client = Client::new();
    let res = client
        .get(url)
        .header(USER_AGENT, UPDATER_USER_AGENT)
        .send()
        .await?;
    let parsed: CapacityBuilderMacrosResponse = res.json().await?;
    parsed
        .versions
        .get(0)
        .ok_or("No versions found")?
        .num
        .clone()
        .parse::<String>()
        .map_err(|_| "Failed to parse version".into())
}

async fn get_remote_dependencies(
    crate_name: &str,
    version: &str,
) -> Result<Vec<Dependency>, Box<dyn std::error::Error>> {
    let url = format!("https://crates.io/api/v1/crates/{crate_name}/{version}/dependencies");
    let client = Client::new();
    let res = client
        .get(url)
        .header(USER_AGENT, UPDATER_USER_AGENT)
        .send()
        .await?;
    let parsed: DependenciesResponse = res.json().await?;
    Ok(parsed.dependencies)
}

fn compare_and_apply_deps(
    doc: &mut DocumentMut,
    deno_deps: &[Dependency],
) -> Result<(), Box<dyn std::error::Error>> {
    if let toml_edit::Item::Table(dep_table) = &mut doc["dependencies"] {
        for (key, val) in dep_table.iter_mut() {
            let dep_name = key.to_string();
            let matched = deno_deps.iter().find(|d| d.crate_id == dep_name);
            if let Some(dep) = matched {
                if let toml_edit::Item::Value(toml_value) = val {
                    update_dependency(dep_name, toml_value, &dep.req)?;
                } else {
                    return Err("Invalid dependency format".into());
                }
            }
        }
    };
    Ok(())
}

fn update_dependency(
    dep_name: String,
    toml_value: &mut toml_edit::Value,
    next_version: &String,
) -> Result<(), Box<dyn std::error::Error>> {
    let version = if toml_value.is_str() {
        toml_value
    } else if toml_value.is_inline_table() {
        toml_value
            .as_inline_table_mut()
            .and_then(|t| t.get_mut("version"))
            .ok_or("Version not found")?
    } else {
        return Err("Invalid dependency format".into());
    };

    if !version.is_str() {
        return Err("Invalid version format".into());
    }

    let version_str = version.as_str().unwrap().to_string();
    if version_str != *next_version {
        println!(
            "Updating {} from {} to {}",
            dep_name, version_str, next_version
        );
        *version = toml_edit::value(next_version).as_value().unwrap().clone();
    }
    Ok(())
}
