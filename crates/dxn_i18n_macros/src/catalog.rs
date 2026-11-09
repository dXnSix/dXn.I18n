//! The crate's messages: where they are, as `[package.metadata.dxn_i18n]` says, and the
//! arguments each one reads, kept while their files do not change.

use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::SystemTime;

use dxn_i18n::Message;
use serde_json::Value as Json;

/// Where the messages are and how their files map to ids.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Settings {
    /// The directory, relative to the crate's `Cargo.toml`.
    pub messages: String,
    /// The locale every id must be in.
    pub default_locale: String,
    /// The namespace whose ids are not prefixed.
    pub default_namespace: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            messages: "locales".to_owned(),
            default_locale: "en".to_owned(),
            default_namespace: None,
        }
    }
}

/// `defaults` (the built-in ones, or a framework's) with what the crate's
/// `[package.metadata.dxn_i18n]` sets instead.
pub(crate) fn settings(manifest_dir: &Path, mut defaults: Settings) -> Result<Settings, String> {
    let manifest = manifest_dir.join("Cargo.toml");
    let text = fs::read_to_string(&manifest)
        .map_err(|error| format!("cannot read `{}`: {error}", manifest.display()))?;
    let table: toml::Table = text
        .parse()
        .map_err(|error| format!("cannot read `{}`: {error}", manifest.display()))?;
    let Some(metadata) = table
        .get("package")
        .and_then(|package| package.get("metadata"))
        .and_then(|metadata| metadata.get("dxn_i18n"))
    else {
        return Ok(defaults);
    };
    let Some(metadata) = metadata.as_table() else {
        return Err("`[package.metadata.dxn_i18n]` in Cargo.toml is not a table".to_owned());
    };
    for (key, value) in metadata {
        let Some(value) = value.as_str() else {
            return Err(format!(
                "`package.metadata.dxn_i18n.{key}` in Cargo.toml is not a string"
            ));
        };
        match key.as_str() {
            "messages" => defaults.messages = value.to_owned(),
            "default-locale" => defaults.default_locale = value.to_owned(),
            "default-namespace" => defaults.default_namespace = Some(value.to_owned()),
            other => {
                return Err(format!(
                    "`package.metadata.dxn_i18n.{other}` in Cargo.toml is no setting; the \
                     settings are `messages`, `default-locale` and `default-namespace`"
                ));
            }
        }
    }
    Ok(defaults)
}

/// A message's file, and the arguments it reads or why it does not load.
#[derive(Debug)]
pub(crate) struct Entry {
    /// The file, as the settings name it (`locales/de.json`).
    pub file: String,
    /// The arguments the message reads, or what is wrong with it (`does not parse: …`).
    pub arguments: Result<Vec<String>, String>,
}

/// Every locale's messages.
#[derive(Debug)]
pub(crate) struct Catalog {
    /// The files read, which the expansion includes so the crate rebuilds when one changes.
    pub files: Vec<PathBuf>,
    /// Each locale's (as its file names it) messages by id.
    pub locales: BTreeMap<String, BTreeMap<String, Entry>>,
}

/// A resource file: its path, its locale and namespace, and what tells a change.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Resource {
    path: PathBuf,
    shown: String,
    locale: String,
    namespace: Option<String>,
    modified: Option<SystemTime>,
    len: u64,
}

type Cache = Mutex<HashMap<(PathBuf, Option<String>), (Vec<Resource>, Arc<Catalog>)>>;

/// The catalog of the messages the settings name, read again only when a file changed.
pub(crate) fn load(manifest_dir: &Path, settings: &Settings) -> Result<Arc<Catalog>, String> {
    static CACHE: std::sync::OnceLock<Cache> = std::sync::OnceLock::new();
    let dir = manifest_dir.join(&settings.messages);
    let resources = resources(&dir, &settings.messages)?;
    let key = (dir, settings.default_namespace.clone());
    let cache = CACHE.get_or_init(Cache::default);
    if let Some((seen, catalog)) = cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .get(&key)
        && *seen == resources
    {
        return Ok(catalog.clone());
    }
    let catalog = Arc::new(read(&resources, settings)?);
    cache
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .insert(key, (resources, catalog.clone()));
    Ok(catalog)
}

/// `<locale>.json` and `<locale>/<namespace>.json` in `dir`, in name order.
fn resources(dir: &Path, shown: &str) -> Result<Vec<Resource>, String> {
    let mut resources = Vec::new();
    let entries = sorted_entries(dir).map_err(|error| {
        format!(
            "no messages directory `{shown}` beside Cargo.toml ({error}); set `messages` in \
             `[package.metadata.dxn_i18n]`"
        )
    })?;
    for path in entries {
        if path.is_dir() {
            let Some(locale) = file_name(&path) else {
                continue;
            };
            let files = sorted_entries(&path)
                .map_err(|error| format!("cannot read `{shown}/{locale}`: {error}"))?;
            for file in files {
                if let Some(namespace) = json_stem(&file) {
                    resources.push(resource(
                        file.clone(),
                        format!("{shown}/{locale}/{namespace}.json"),
                        locale.clone(),
                        Some(namespace),
                    )?);
                }
            }
        } else if let Some(locale) = json_stem(&path) {
            resources.push(resource(
                path.clone(),
                format!("{shown}/{locale}.json"),
                locale,
                None,
            )?);
        }
    }
    Ok(resources)
}

fn resource(
    path: PathBuf,
    shown: String,
    locale: String,
    namespace: Option<String>,
) -> Result<Resource, String> {
    let metadata =
        fs::metadata(&path).map_err(|error| format!("cannot read `{shown}`: {error}"))?;
    Ok(Resource {
        path,
        shown,
        locale,
        namespace,
        modified: metadata.modified().ok(),
        len: metadata.len(),
    })
}

fn sorted_entries(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut entries = fs::read_dir(dir)?
        .map(|entry| entry.map(|entry| entry.path()))
        .collect::<std::io::Result<Vec<_>>>()?;
    entries.sort();
    Ok(entries)
}

fn file_name(path: &Path) -> Option<String> {
    path.file_name()?.to_str().map(str::to_owned)
}

fn json_stem(path: &Path) -> Option<String> {
    if !path.is_file() || path.extension()? != "json" {
        return None;
    }
    path.file_stem()?.to_str().map(str::to_owned)
}

fn read(resources: &[Resource], settings: &Settings) -> Result<Catalog, String> {
    let mut catalog = Catalog {
        files: Vec::new(),
        locales: BTreeMap::new(),
    };
    for resource in resources {
        let text = fs::read_to_string(&resource.path)
            .map_err(|error| format!("cannot read `{}`: {error}", resource.shown))?;
        let root: Json = serde_json::from_str(&text).map_err(|error| {
            format!(
                "`{}` is not a JSON object of messages: {error}",
                resource.shown
            )
        })?;
        if !root.is_object() {
            return Err(format!(
                "`{}` is not a JSON object of messages",
                resource.shown
            ));
        }
        let prefix = match &resource.namespace {
            Some(namespace) if Some(namespace) != settings.default_namespace.as_ref() => {
                namespace.as_str()
            }
            _ => "",
        };
        let messages = catalog.locales.entry(resource.locale.clone()).or_default();
        walk(prefix, &root, &resource.shown, messages);
        catalog.files.push(resource.path.clone());
    }
    Ok(catalog)
}

/// As `Bundle::add_json` reads a resource: nested objects are dotted ids, text is a message.
fn walk(id: &str, node: &Json, file: &str, messages: &mut BTreeMap<String, Entry>) {
    match node {
        Json::Object(entries) => {
            for (key, child) in entries {
                let child_id = if id.is_empty() {
                    key.clone()
                } else {
                    format!("{id}.{key}")
                };
                walk(&child_id, child, file, messages);
            }
        }
        Json::String(source) => {
            let arguments = Message::parse(source)
                .map(|message| message.arguments())
                .map_err(|error| format!("does not parse: {error}"));
            messages.insert(
                id.to_owned(),
                Entry {
                    file: file.to_owned(),
                    arguments,
                },
            );
        }
        _ => {
            messages.insert(
                id.to_owned(),
                Entry {
                    file: file.to_owned(),
                    arguments: Err("is not text".to_owned()),
                },
            );
        }
    }
}

/// Whether two locale names are the same locale (`en-US`, `en_us`).
pub(crate) fn same_locale(a: &str, b: &str) -> bool {
    a.replace('_', "-")
        .eq_ignore_ascii_case(&b.replace('_', "-"))
}
