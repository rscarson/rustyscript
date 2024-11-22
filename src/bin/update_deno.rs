//!
//! Designed to automatically collect required versions of deno crates from the latest version of the `deno_runtime` crate.
//! Needed due to the sheer number of dependencies needed to be updated in `Cargo.toml`.
//!
//! Built so that relevant sections can just be pasted into `Cargo.toml` directly.
use deno_core::{anyhow, serde_json};
use std::collections::HashMap;

fn main() {
    match get_relevant_versions() {
        Ok(deps) => {
            println!("\n\n{}", format_version_blocks(&deps));
        }
        Err(e) => {
            eprintln!("Error: {:?}", e);
        }
    }
}

fn get_crates_api_data(name: &str) -> Result<serde_json::Value, anyhow::Error> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("rustyscript_updatebot (https://github.com/rscarson/rustyscript)")
        .danger_accept_invalid_certs(true)
        .build()?;

    // Get the first 4 alphanumeric characters of the name
    // So deno_runtime becomes de/no
    let prefix = name.chars().take(4).collect::<String>();
    let p1 = &prefix[..2];
    let p2 = &prefix[2..4];

    // Version first
    let request = client.get(&format!("https://index.crates.io/{p1}/{p2}/{name}"));
    let response = request.send()?.text()?;

    // Only the last line
    let last_line = response.lines().last().unwrap();
    Ok(serde_json::from_str(last_line)?)
}

/// Returns the set of dependencies for the latest version of the `deno_runtime` crate
fn get_deno_runtime_dependencies() -> Result<DependencyMap, anyhow::Error> {
    let json = get_crates_api_data("deno_runtime")?;

    let mut deps = DependencyMap::new();
    deps.insert(
        "deno_runtime".to_string(),
        DependencyState::from_version(
            json["vers"]
                .as_str()
                .expect("Malformed deno_runtime version"),
        ),
    );
    for dep in json["deps"]
        .as_array()
        .expect("Malformed deno_runtime dependencies")
    {
        let name = dep["name"]
            .as_str()
            .expect("Malformed deno_runtime dependency name")
            .to_string();
        let version = dep["req"]
            .as_str()
            .expect("Malformed deno_runtime dependency version")
            .trim_start_matches("^");

        deps.insert(name, DependencyState::from_version(version));
    }

    // Get deno_resolver seperately since the rt crate doesn't have it
    let json = get_crates_api_data("deno_resolver")?;
    deps.insert(
        "deno_resolver".to_string(),
        DependencyState::from_version(
            json["vers"]
                .as_str()
                .expect("Malformed deno_resolver version"),
        ),
    );

    Ok(deps)
}

fn get_own_dependencies() -> Result<DependencyMap, anyhow::Error> {
    let cargo_toml = std::fs::read_to_string("Cargo.toml")?;
    let toml: toml::Value = toml::from_str(&cargo_toml)?;

    let mut deps = DependencyMap::new();
    for (name, entry) in toml["dependencies"]
        .as_table()
        .expect("Malformed Cargo.toml - dependencies")
    {
        deps.insert(name.to_string(), DependencyState::from_toml(entry));
    }

    Ok(deps)
}

fn crossref_dependencies(
    own_deps: &DependencyMap,
    deno_runtime_deps: &DependencyMap,
) -> DependencyMap {
    let mut updated_deps = own_deps.clone();
    for (name, state) in own_deps.iter() {
        let version = &state.version;
        if let Some(deno_version) = deno_runtime_deps.maybe_version(name) {
            if version != deno_version {
                println!("^ {name}: {version} -> {deno_version}");
                updated_deps.update(name.clone(), deno_version.clone());
            }
        }
    }

    updated_deps
}

fn get_relevant_versions() -> Result<DependencyMap, anyhow::Error> {
    let own_deps = get_own_dependencies()?;
    let deno_runtime_deps = get_deno_runtime_dependencies()?;

    Ok(crossref_dependencies(&own_deps, &deno_runtime_deps))
}

#[derive(Clone)]
struct DependencyState {
    version: String,
    updated: bool,
    optional: bool,
    no_default_features: bool,
    features: Vec<String>,
}
impl DependencyState {
    pub fn from_version(version: &str) -> Self {
        Self {
            version: version.to_string(),
            updated: false,
            optional: false,
            no_default_features: false,
            features: vec![],
        }
    }

    pub fn from_toml(entry: &toml::Value) -> Self {
        match entry.as_str() {
            Some(v) => Self::from_version(v),
            None => {
                let version = entry["version"]
                    .as_str()
                    .expect("Malformed dependency entry - version");
                let optional = entry
                    .get("optional")
                    .map_or(false, |v| v.as_bool().unwrap());
                let no_default_features = entry
                    .get("default-features")
                    .map_or(false, |v| !v.as_bool().unwrap());
                let features = entry.get("features").map_or(vec![], |v| {
                    v.as_array()
                        .unwrap()
                        .iter()
                        .map(|v| format!("\"{}\"", v.as_str().unwrap()))
                        .collect()
                });

                Self {
                    version: version.to_string(),
                    updated: false,
                    optional,
                    no_default_features,
                    features,
                }
            }
        }
    }
}

#[derive(Clone)]
struct DependencyMap(HashMap<String, DependencyState>);
impl DependencyMap {
    pub fn new() -> Self {
        Self(HashMap::new())
    }

    pub fn insert(&mut self, name: String, state: DependencyState) {
        self.0.insert(name, state);
    }

    pub fn update(&mut self, name: String, version: String) {
        if let Some(state) = self.0.get_mut(&name) {
            state.version = version;
            state.updated = true;
        }
    }

    pub fn maybe_version(&self, name: &str) -> Option<&String> {
        self.0.get(name).map(|s| &s.version)
    }

    pub fn get_or_die(&self, name: &str) -> &DependencyState {
        self.0
            .get(name)
            .unwrap_or_else(|| panic!("Dependency {name} not found"))
    }

    pub fn iter(&self) -> std::collections::hash_map::Iter<String, DependencyState> {
        self.0.iter()
    }

    pub fn fmt(&self, name: &str) -> String {
        self.fmt_with_name_pad(name, 0)
    }

    pub fn fmt_with_name_pad(&self, name: &str, min_name_len: usize) -> String {
        let state = self.get_or_die(name);
        let version = &state.version;

        let mut name = name.to_string();
        if name.len() < min_name_len {
            name.push_str(&" ".repeat(min_name_len - name.len()));
        }

        // Escape code for green color foreground
        let grn_color_hex = if state.updated { "\x1b[32m" } else { "" };
        let reset_hex = if state.updated { "\x1b[0m" } else { "" };

        let out = if !state.optional && !state.no_default_features && state.features.is_empty() {
            format!(r#""{version}""#)
        } else {
            let mut out = format!(r#"{{ version = "{version}""#, version = version);
            if state.optional {
                out.push_str(", optional = true");
            }
            if state.no_default_features {
                out.push_str(", default-features = false");
            }
            if !state.features.is_empty() {
                out.push_str(", features = [");
                out.push_str(&state.features.join(", "));
                out.push(']');
            }
            out.push_str(" }");
            out
        };

        format!("{grn_color_hex}{name} = {out}{reset_hex}")
    }
}

fn format_version_blocks(map: &DependencyMap) -> String {
    let mut output = vec![];

    //
    // Core deps
    output.extend([
        map.fmt("deno_core"),
        map.fmt("deno_ast"),
        String::new(),
        //
        map.fmt("reqwest"),
        map.fmt("http"),
        map.fmt("deno_permissions"),
        String::new(),
    ]);

    output.push("\n---------------------\n".to_string());

    //
    // Extensions deps
    output.extend([
        map.fmt("deno_broadcast_channel"),
        String::new(),
        //
        map.fmt_with_name_pad("deno_cache", 15),
        map.fmt_with_name_pad("deno_console", 15),
        map.fmt_with_name_pad("deno_cron", 15),
        map.fmt_with_name_pad("deno_crypto", 15),
        map.fmt_with_name_pad("deno_fetch", 15),
        map.fmt_with_name_pad("deno_ffi", 15),
        map.fmt_with_name_pad("deno_fs", 15),
        map.fmt_with_name_pad("deno_http", 15),
        map.fmt_with_name_pad("deno_kv", 15),
        map.fmt_with_name_pad("deno_net", 15),
        map.fmt_with_name_pad("deno_node", 15),
        map.fmt_with_name_pad("deno_tls", 15),
        map.fmt_with_name_pad("deno_url", 15),
        String::new(),
        //
        map.fmt_with_name_pad("deno_web", 15),
        map.fmt_with_name_pad("deno_webidl", 15),
        map.fmt_with_name_pad("deno_webstorage", 15),
        map.fmt_with_name_pad("deno_websocket", 15),
        map.fmt_with_name_pad("deno_webgpu", 15),
        String::new(),
        //
        map.fmt("deno_io"),
    ]);

    output.push("\n---------------------\n".to_string());

    //
    // Node deps
    output.extend([
        map.fmt_with_name_pad("deno_resolver", 13),
        map.fmt_with_name_pad("node_resolver", 13),
        map.fmt_with_name_pad("deno_runtime", 13),
        map.fmt_with_name_pad("deno_terminal", 13),
        map.fmt_with_name_pad("deno_semver", 13),
        map.fmt_with_name_pad("deno_napi", 13),
        map.fmt_with_name_pad("deno_npm", 13),
        map.fmt_with_name_pad("checksum", 13),
    ]);

    output.join("\n")
}
