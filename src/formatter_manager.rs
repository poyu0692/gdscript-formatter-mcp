use reqwest::blocking::Client;
use reqwest::header::{ACCEPT, USER_AGENT};
use serde::Deserialize;
use std::env;
use std::fs::{self, File};
use std::io;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::tempdir_in;
use zip::ZipArchive;

pub const SERVER_NAME: &str = "gdscript-formatter-mcp";
const LATEST_RELEASE_API_URL: &str =
    "https://api.github.com/repos/GDQuest/GDScript-formatter/releases/latest";

#[derive(Debug, Deserialize)]
struct ReleaseInfo {
    tag_name: String,
    assets: Vec<ReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct ReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Clone, Debug)]
struct PlatformInfo {
    os: String,
    arch: String,
    binary_name: String,
}

pub struct FormatterManager {
    cache_root: PathBuf,
    platform: Option<PlatformInfo>,
    client: Client,
}

impl FormatterManager {
    pub fn new() -> Result<Self, String> {
        let platform = detect_platform();
        let cache_root = resolve_cache_root()?;

        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

        Ok(Self {
            cache_root,
            platform,
            client,
        })
    }

    pub fn ensure_binary(&self) -> Result<PathBuf, String> {
        if let Some(path) = env::var_os("GDSCRIPT_FORMATTER_PATH") {
            let path = PathBuf::from(path);
            if path.exists() {
                return Ok(path);
            }
            return Err(format!(
                "GDSCRIPT_FORMATTER_PATH points to a missing file: {}",
                path.display()
            ));
        }

        let platform = self.platform.as_ref().ok_or_else(|| {
            format!(
                "Unsupported platform for gdscript-formatter: os={} arch={}",
                env::consts::OS,
                env::consts::ARCH
            )
        })?;

        let platform_dir = self
            .cache_root
            .join(format!("{}-{}", platform.os, platform.arch));
        fs::create_dir_all(&platform_dir).map_err(|e| {
            format!(
                "Failed to create platform cache dir {}: {}",
                platform_dir.display(),
                e
            )
        })?;

        let binary_path = platform_dir.join(&platform.binary_name);
        let version_file_path = platform_dir.join("VERSION");

        match self.fetch_latest_release() {
            Ok(release) => {
                let update_result = (|| -> Result<(), String> {
                    let asset = select_asset_for_platform(&release, platform)?;
                    let installed_tag = fs::read_to_string(&version_file_path)
                        .ok()
                        .map(|s| s.trim().to_owned());

                    if installed_tag.as_deref() == Some(release.tag_name.as_str())
                        && binary_path.exists()
                    {
                        return Ok(());
                    }

                    self.download_and_extract_asset(&asset.browser_download_url, &binary_path)?;
                    fs::write(&version_file_path, format!("{}\n", release.tag_name)).map_err(
                        |e| {
                            format!(
                                "Failed to write version file {}: {}",
                                version_file_path.display(),
                                e
                            )
                        },
                    )?;
                    Ok(())
                })();

                match update_result {
                    Ok(()) => Ok(binary_path),
                    Err(update_err) => {
                        if binary_path.exists() {
                            eprintln!(
                                "Warning: could not update formatter, using cached binary: {update_err}"
                            );
                            Ok(binary_path)
                        } else {
                            Err(format!(
                                "Failed to update formatter and no cached formatter found: {update_err}"
                            ))
                        }
                    }
                }
            }
            Err(fetch_err) => {
                if binary_path.exists() {
                    eprintln!(
                        "Warning: could not fetch latest release, using cached formatter: {fetch_err}"
                    );
                    Ok(binary_path)
                } else {
                    Err(format!(
                        "Failed to fetch latest release and no cached formatter found: {fetch_err}"
                    ))
                }
            }
        }
    }

    fn fetch_latest_release(&self) -> Result<ReleaseInfo, String> {
        self.client
            .get(LATEST_RELEASE_API_URL)
            .header(
                USER_AGENT,
                format!("{}/{}", SERVER_NAME, env!("CARGO_PKG_VERSION")),
            )
            .header(ACCEPT, "application/vnd.github+json")
            .send()
            .map_err(|e| format!("HTTP request to GitHub failed: {e}"))?
            .error_for_status()
            .map_err(|e| format!("GitHub latest release request failed: {e}"))?
            .json::<ReleaseInfo>()
            .map_err(|e| format!("Failed to parse GitHub release JSON: {e}"))
    }

    fn download_and_extract_asset(
        &self,
        url: &str,
        target_binary_path: &Path,
    ) -> Result<(), String> {
        let response = self
            .client
            .get(url)
            .header(
                USER_AGENT,
                format!("{}/{}", SERVER_NAME, env!("CARGO_PKG_VERSION")),
            )
            .send()
            .map_err(|e| format!("Failed to download asset from {url}: {e}"))?
            .error_for_status()
            .map_err(|e| format!("Asset download failed: {e}"))?;

        let bytes = response
            .bytes()
            .map_err(|e| format!("Failed to read asset bytes: {e}"))?;

        let temp_dir = tempdir_in(&self.cache_root)
            .map_err(|e| format!("Failed to create temp dir in cache: {e}"))?;
        let zip_path = temp_dir.path().join("asset.zip");
        fs::write(&zip_path, &bytes).map_err(|e| {
            format!(
                "Failed to write downloaded zip to {}: {}",
                zip_path.display(),
                e
            )
        })?;

        let file = File::open(&zip_path).map_err(|e| {
            format!(
                "Failed to open downloaded zip {}: {}",
                zip_path.display(),
                e
            )
        })?;
        let mut archive =
            ZipArchive::new(file).map_err(|e| format!("Failed to read zip archive: {e}"))?;

        let expected_binary_name = target_binary_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| {
                format!(
                    "Failed to determine expected binary filename from {}",
                    target_binary_path.display()
                )
            })?;

        let mut extracted = false;
        for i in 0..archive.len() {
            let mut entry = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read zip entry #{i}: {e}"))?;
            if entry.is_dir() {
                continue;
            }
            let is_expected_binary = Path::new(entry.name())
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n == expected_binary_name)
                .unwrap_or(false);
            if !is_expected_binary {
                continue;
            }

            let temp_output = target_binary_path.with_extension("download");
            let mut out_file = File::create(&temp_output).map_err(|e| {
                format!(
                    "Failed to create temporary binary {}: {}",
                    temp_output.display(),
                    e
                )
            })?;

            io::copy(&mut entry, &mut out_file)
                .map_err(|e| format!("Failed to extract formatter binary: {e}"))?;

            set_executable_permissions(&temp_output)?;
            fs::rename(&temp_output, target_binary_path).map_err(|e| {
                format!(
                    "Failed to move binary into place {}: {}",
                    target_binary_path.display(),
                    e
                )
            })?;

            extracted = true;
            break;
        }

        if !extracted {
            return Err(format!(
                "Formatter binary '{}' not found in downloaded zip asset",
                expected_binary_name
            ));
        }

        Ok(())
    }
}

fn detect_platform() -> Option<PlatformInfo> {
    let os = match env::consts::OS {
        "linux" => "linux",
        "macos" => "macos",
        "windows" => "windows",
        _ => return None,
    };

    let arch = match env::consts::ARCH {
        "x86_64" => "x86_64",
        "aarch64" => "aarch64",
        _ => return None,
    };

    let binary_name = if os == "windows" {
        "gdscript-formatter.exe"
    } else {
        "gdscript-formatter"
    };

    Some(PlatformInfo {
        os: os.to_owned(),
        arch: arch.to_owned(),
        binary_name: binary_name.to_owned(),
    })
}

fn default_cache_root() -> PathBuf {
    if let Some(xdg_cache_home) = env::var_os("XDG_CACHE_HOME") {
        return PathBuf::from(xdg_cache_home).join(SERVER_NAME);
    }
    if let Some(home) = env::var_os("HOME") {
        return PathBuf::from(home).join(".cache").join(SERVER_NAME);
    }
    PathBuf::from(".gdscript-formatter-mcp-cache")
}

fn resolve_cache_root() -> Result<PathBuf, String> {
    if let Some(custom) = env::var_os("GDSCRIPT_FORMATTER_MCP_CACHE_DIR") {
        let path = PathBuf::from(custom);
        fs::create_dir_all(&path).map_err(|e| {
            format!(
                "Failed to create custom cache dir {} from GDSCRIPT_FORMATTER_MCP_CACHE_DIR: {}",
                path.display(),
                e
            )
        })?;
        return Ok(path);
    }

    let mut candidates = Vec::new();
    candidates.push(default_cache_root());
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd.join(".gdscript-formatter-mcp-cache"));
    }
    candidates.push(env::temp_dir().join(SERVER_NAME));

    let mut errors = Vec::new();
    for candidate in candidates {
        match fs::create_dir_all(&candidate) {
            Ok(_) => return Ok(candidate),
            Err(err) => errors.push(format!("{} ({})", candidate.display(), err)),
        }
    }

    Err(format!(
        "Unable to create any cache directory. Tried: {}",
        errors.join(", ")
    ))
}

fn select_asset_for_platform<'a>(
    release: &'a ReleaseInfo,
    platform: &PlatformInfo,
) -> Result<&'a ReleaseAsset, String> {
    let needle = format!("-{}-{}", platform.os, platform.arch);
    release
        .assets
        .iter()
        .find(|asset| {
            asset.name.starts_with("gdscript-formatter-")
                && asset.name.contains(&needle)
                && asset.name.ends_with(".zip")
        })
        .ok_or_else(|| {
            format!(
                "No downloadable formatter asset found for {}-{} in release {}",
                platform.os, platform.arch, release.tag_name
            )
        })
}

#[cfg(unix)]
fn set_executable_permissions(path: &Path) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let perms = fs::Permissions::from_mode(0o755);
    fs::set_permissions(path, perms).map_err(|e| {
        format!(
            "Failed to set executable permissions {}: {}",
            path.display(),
            e
        )
    })
}

#[cfg(not(unix))]
fn set_executable_permissions(_path: &Path) -> Result<(), String> {
    Ok(())
}
