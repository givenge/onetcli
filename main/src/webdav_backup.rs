use std::fs;
use std::io::BufReader;
use std::path::{Component, Path, PathBuf};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use chrono::{DateTime, Utc};
use flate2::read::GzDecoder;
use flate2::{Compression, write::GzEncoder};
use futures::AsyncReadExt;
use gpui::http_client::{AsyncBody, HttpClient, Method, Request, StatusCode};
use one_core::settings::WebDavBackupSettings;
use quick_xml::Reader;
use quick_xml::events::Event;
use serde::{Deserialize, Serialize};

const DEFAULT_REMOTE_DIR: &str = "onetcli-backup";
const BACKUP_FILE_PREFIX: &str = "onetcli-backup";
const BACKUP_USER_AGENT: &str = "onetcli-webdav-backup";
const SKIPPED_CONFIG_DIRS: &[&str] = &["logs"];
const RESTORE_STAGING_DIR: &str = ".webdav-restore";
const STAGED_RESTORE_ARCHIVE: &str = "pending-restore.tar.gz";
const STAGED_RESTORE_META: &str = "pending-restore.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebDavBackupResult {
    pub file_name: String,
    pub target_url: String,
    pub bytes: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteBackupEntry {
    pub file_name: String,
    pub modified_at: i64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StagedRestoreResult {
    pub file_name: String,
    pub staged_path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppliedRestoreResult {
    pub file_name: String,
    pub restored_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct StagedRestoreMeta {
    file_name: String,
    staged_at: i64,
}

pub async fn test_webdav_connection(
    settings: WebDavBackupSettings,
    http_client: Arc<dyn HttpClient>,
) -> Result<(), String> {
    settings.validate_endpoint()?;
    ensure_remote_collection(&settings, http_client).await
}

pub async fn backup_to_webdav(
    settings: WebDavBackupSettings,
    http_client: Arc<dyn HttpClient>,
    config_dir: PathBuf,
) -> Result<WebDavBackupResult, String> {
    settings.validate_for_backup()?;
    ensure_remote_collection(&settings, http_client.clone()).await?;

    let timestamp = current_timestamp();
    let file_name = backup_filename_from_timestamp(timestamp);
    let target_url = build_backup_target_url(&settings.endpoint, &settings.remote_dir, &file_name)?;
    let archive = build_config_archive(&config_dir)?;
    let bytes = archive.len();

    upload_backup(&settings, http_client, &target_url, archive).await?;

    Ok(WebDavBackupResult {
        file_name,
        target_url,
        bytes,
    })
}

pub async fn list_backups(
    settings: WebDavBackupSettings,
    http_client: Arc<dyn HttpClient>,
) -> Result<Vec<RemoteBackupEntry>, String> {
    settings.validate_for_backup()?;
    ensure_remote_collection(&settings, http_client.clone()).await?;

    let collection_url = remote_dir_url(&settings)?;
    let body = br#"<?xml version="1.0" encoding="utf-8"?><d:propfind xmlns:d="DAV:"><d:prop><d:getlastmodified/><d:getcontentlength/></d:prop></d:propfind>"#.to_vec();
    let (status, bytes) = send_webdav_request_with_headers(
        &settings,
        http_client,
        propfind_method()?,
        &collection_url,
        Some(body),
        &[
            ("Depth", "1"),
            ("Content-Type", "application/xml; charset=utf-8"),
        ],
    )
    .await?;

    if status != multi_status_code() && !status.is_success() {
        return Err(format!(
            "获取 WebDAV 备份列表失败：{} {}",
            status,
            String::from_utf8_lossy(&bytes)
        ));
    }

    parse_propfind_backups(&bytes)
}

pub async fn delete_backup(
    settings: WebDavBackupSettings,
    http_client: Arc<dyn HttpClient>,
    file_name: &str,
) -> Result<(), String> {
    settings.validate_for_backup()?;
    if !is_backup_archive_name(file_name) {
        return Err("不支持的备份文件名".to_string());
    }

    let target_url = build_backup_target_url(&settings.endpoint, &settings.remote_dir, file_name)?;
    let (status, bytes) =
        send_webdav_request(&settings, http_client, Method::DELETE, &target_url, None).await?;
    if status.is_success() {
        Ok(())
    } else {
        Err(format!(
            "删除 WebDAV 备份失败：{} {}",
            status,
            String::from_utf8_lossy(&bytes)
        ))
    }
}

pub async fn stage_restore_from_webdav(
    settings: WebDavBackupSettings,
    http_client: Arc<dyn HttpClient>,
    config_dir: PathBuf,
    file_name: &str,
) -> Result<StagedRestoreResult, String> {
    settings.validate_for_backup()?;
    if !is_backup_archive_name(file_name) {
        return Err("不支持的备份文件名".to_string());
    }

    let target_url = build_backup_target_url(&settings.endpoint, &settings.remote_dir, file_name)?;
    let (status, bytes) =
        send_webdav_request(&settings, http_client, Method::GET, &target_url, None).await?;
    if !status.is_success() {
        return Err(format!(
            "下载 WebDAV 备份失败：{} {}",
            status,
            String::from_utf8_lossy(&bytes)
        ));
    }

    let staging_dir = staged_restore_dir(&config_dir);
    fs::create_dir_all(&staging_dir).map_err(|err| format!("创建恢复暂存目录失败: {}", err))?;
    let staged_path = staged_restore_archive_path(&config_dir);
    fs::write(&staged_path, &bytes).map_err(|err| format!("写入恢复暂存包失败: {}", err))?;

    let meta = StagedRestoreMeta {
        file_name: file_name.to_string(),
        staged_at: current_timestamp(),
    };
    let meta_bytes =
        serde_json::to_vec_pretty(&meta).map_err(|err| format!("序列化恢复元数据失败: {}", err))?;
    fs::write(staged_restore_meta_path(&config_dir), meta_bytes)
        .map_err(|err| format!("写入恢复元数据失败: {}", err))?;

    Ok(StagedRestoreResult {
        file_name: file_name.to_string(),
        staged_path,
    })
}

pub fn apply_pending_restore(config_dir: &Path) -> Result<Option<AppliedRestoreResult>, String> {
    let archive_path = staged_restore_archive_path(config_dir);
    if !archive_path.exists() {
        return Ok(None);
    }

    let meta = read_staged_restore_meta(config_dir).unwrap_or(StagedRestoreMeta {
        file_name: archive_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(STAGED_RESTORE_ARCHIVE)
            .to_string(),
        staged_at: current_timestamp(),
    });

    let extract_dir = staged_restore_dir(config_dir).join("extract");
    if extract_dir.exists() {
        fs::remove_dir_all(&extract_dir).map_err(|err| format!("清理旧恢复目录失败: {}", err))?;
    }
    fs::create_dir_all(&extract_dir).map_err(|err| format!("创建恢复目录失败: {}", err))?;

    extract_restore_archive(&archive_path, &extract_dir)?;
    let restored_files = copy_restore_tree(&extract_dir, config_dir)?;

    cleanup_restore_staging(config_dir)?;

    Ok(Some(AppliedRestoreResult {
        file_name: meta.file_name,
        restored_files,
    }))
}

pub(crate) fn normalize_remote_dir(input: &str) -> String {
    let trimmed = input.trim().trim_matches('/');
    if trimmed.is_empty() {
        DEFAULT_REMOTE_DIR.to_string()
    } else {
        trimmed.to_string()
    }
}

pub(crate) fn backup_filename_from_timestamp(timestamp: i64) -> String {
    let dt = DateTime::<Utc>::from_timestamp(timestamp, 0).unwrap_or(DateTime::<Utc>::UNIX_EPOCH);
    format!(
        "{}-{}.tar.gz",
        BACKUP_FILE_PREFIX,
        dt.format("%Y%m%d-%H%M%S")
    )
}

pub(crate) fn build_backup_target_url(
    endpoint: &str,
    remote_dir: &str,
    file_name: &str,
) -> Result<String, String> {
    let endpoint = normalized_endpoint(endpoint)?;
    let remote_dir = normalize_remote_dir(remote_dir);
    Ok(format!("{endpoint}/{remote_dir}/{file_name}"))
}

fn current_timestamp() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn normalized_endpoint(endpoint: &str) -> Result<String, String> {
    let endpoint = endpoint.trim().trim_end_matches('/');
    if endpoint.is_empty() {
        Err("WebDAV 地址不能为空".to_string())
    } else {
        Ok(endpoint.to_string())
    }
}

fn remote_dir_url(settings: &WebDavBackupSettings) -> Result<String, String> {
    let endpoint = normalized_endpoint(&settings.endpoint)?;
    let remote_dir = normalize_remote_dir(&settings.remote_dir);
    Ok(format!("{endpoint}/{remote_dir}"))
}

fn remote_collection_urls(settings: &WebDavBackupSettings) -> Result<Vec<String>, String> {
    let endpoint = normalized_endpoint(&settings.endpoint)?;
    let mut urls = Vec::new();
    let mut current = endpoint;
    for segment in normalize_remote_dir(&settings.remote_dir).split('/') {
        if segment.is_empty() {
            continue;
        }
        current = format!("{current}/{segment}");
        urls.push(current.clone());
    }
    Ok(urls)
}

async fn ensure_remote_collection(
    settings: &WebDavBackupSettings,
    http_client: Arc<dyn HttpClient>,
) -> Result<(), String> {
    for url in remote_collection_urls(settings)? {
        let status =
            send_webdav_request(settings, http_client.clone(), mkcol_method()?, &url, None)
                .await?
                .0;
        if !is_mkcol_ok(status) {
            return Err(format!("创建 WebDAV 目录失败：{} ({})", url, status));
        }
    }
    Ok(())
}

fn parse_propfind_backups(xml: &[u8]) -> Result<Vec<RemoteBackupEntry>, String> {
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Field {
        None,
        Href,
        Modified,
        Length,
    }

    let mut reader = Reader::from_reader(xml);
    reader.config_mut().trim_text(true);

    let mut buf = Vec::new();
    let mut field = Field::None;
    let mut in_response = false;
    let mut current_href: Option<String> = None;
    let mut current_modified_at: Option<i64> = None;
    let mut current_length: Option<u64> = None;
    let mut entries = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(event)) => {
                let tag = local_name(event.name().as_ref()).to_string();
                if tag == "response" {
                    in_response = true;
                    current_href = None;
                    current_modified_at = None;
                    current_length = None;
                    field = Field::None;
                } else if in_response {
                    field = match tag.as_str() {
                        "href" => Field::Href,
                        "getlastmodified" => Field::Modified,
                        "getcontentlength" => Field::Length,
                        _ => Field::None,
                    };
                }
            }
            Ok(Event::Text(event)) => {
                if !in_response {
                    buf.clear();
                    continue;
                }
                let text = String::from_utf8_lossy(event.as_ref()).trim().to_string();
                match field {
                    Field::Href => current_href = Some(text),
                    Field::Modified => {
                        current_modified_at = parse_webdav_modified_at(&text);
                    }
                    Field::Length => {
                        current_length = text.parse::<u64>().ok();
                    }
                    Field::None => {}
                }
            }
            Ok(Event::End(event)) => {
                let tag = local_name(event.name().as_ref()).to_string();
                if tag == "response" {
                    if let Some(entry) =
                        build_backup_entry(current_href.take(), current_modified_at, current_length)
                    {
                        entries.push(entry);
                    }
                    in_response = false;
                }
                field = Field::None;
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(err) => return Err(format!("解析 WebDAV 备份列表失败: {}", err)),
        }
        buf.clear();
    }

    entries.sort_by(|a, b| {
        b.modified_at
            .cmp(&a.modified_at)
            .then_with(|| b.file_name.cmp(&a.file_name))
    });
    Ok(entries)
}

async fn upload_backup(
    settings: &WebDavBackupSettings,
    http_client: Arc<dyn HttpClient>,
    target_url: &str,
    archive: Vec<u8>,
) -> Result<(), String> {
    let (status, body) = send_webdav_request(
        settings,
        http_client,
        Method::PUT,
        target_url,
        Some(archive),
    )
    .await?;
    if status.is_success() || status == StatusCode::CREATED {
        return Ok(());
    }
    Err(format!(
        "上传 WebDAV 备份失败：{} {}",
        status,
        String::from_utf8_lossy(&body)
    ))
}

async fn send_webdav_request(
    settings: &WebDavBackupSettings,
    http_client: Arc<dyn HttpClient>,
    method: Method,
    url: &str,
    body: Option<Vec<u8>>,
) -> Result<(StatusCode, Vec<u8>), String> {
    send_webdav_request_with_headers(settings, http_client, method, url, body, &[]).await
}

async fn send_webdav_request_with_headers(
    settings: &WebDavBackupSettings,
    http_client: Arc<dyn HttpClient>,
    method: Method,
    url: &str,
    body: Option<Vec<u8>>,
    headers: &[(&str, &str)],
) -> Result<(StatusCode, Vec<u8>), String> {
    let mut builder = Request::builder()
        .method(method)
        .uri(url)
        .header("User-Agent", BACKUP_USER_AGENT);
    for (key, value) in headers {
        builder = builder.header(*key, *value);
    }
    builder = add_auth_header(builder, settings);
    let request = builder
        .body(body.map(AsyncBody::from).unwrap_or_else(AsyncBody::empty))
        .map_err(|err| format!("构建 WebDAV 请求失败: {}", err))?;

    let response = http_client
        .send(request)
        .await
        .map_err(|err| format!("WebDAV 请求失败: {}", err))?;
    let status = response.status();
    let mut body = response.into_body();
    let mut bytes = Vec::new();
    body.read_to_end(&mut bytes)
        .await
        .map_err(|err| format!("读取 WebDAV 响应失败: {}", err))?;
    Ok((status, bytes))
}

fn add_auth_header(
    builder: gpui::http_client::http::request::Builder,
    settings: &WebDavBackupSettings,
) -> gpui::http_client::http::request::Builder {
    if settings.username.trim().is_empty() && settings.password.is_empty() {
        return builder;
    }
    let token = BASE64.encode(format!(
        "{}:{}",
        settings.username.trim(),
        settings.password
    ));
    builder.header("Authorization", format!("Basic {token}"))
}

fn mkcol_method() -> Result<Method, String> {
    Method::from_bytes(b"MKCOL").map_err(|err| format!("构建 WebDAV MKCOL 方法失败: {}", err))
}

fn propfind_method() -> Result<Method, String> {
    Method::from_bytes(b"PROPFIND").map_err(|err| format!("构建 WebDAV PROPFIND 方法失败: {}", err))
}

fn is_mkcol_ok(status: StatusCode) -> bool {
    status.is_success() || status == StatusCode::METHOD_NOT_ALLOWED
}

fn multi_status_code() -> StatusCode {
    StatusCode::from_u16(207).expect("207 should be a valid status code")
}

fn build_config_archive(config_dir: &Path) -> Result<Vec<u8>, String> {
    let encoder = GzEncoder::new(Vec::new(), Compression::default());
    let mut builder = tar::Builder::new(encoder);
    append_config_dir(&mut builder, config_dir, config_dir)?;
    let encoder = builder
        .into_inner()
        .map_err(|err| format!("完成备份归档失败: {}", err))?;
    encoder
        .finish()
        .map_err(|err| format!("压缩备份归档失败: {}", err))
}

fn append_config_dir(
    builder: &mut tar::Builder<GzEncoder<Vec<u8>>>,
    base: &Path,
    dir: &Path,
) -> Result<(), String> {
    let entries = std::fs::read_dir(dir).map_err(|err| format!("读取配置目录失败: {}", err))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("读取配置项失败: {}", err))?;
        append_config_path(builder, base, entry.path())?;
    }
    Ok(())
}

fn append_config_path(
    builder: &mut tar::Builder<GzEncoder<Vec<u8>>>,
    base: &Path,
    path: PathBuf,
) -> Result<(), String> {
    if should_skip_path(&path) {
        return Ok(());
    }
    if path.is_dir() {
        append_config_dir(builder, base, &path)?;
    } else if path.is_file() {
        let rel = path
            .strip_prefix(base)
            .map_err(|err| format!("计算备份路径失败: {}", err))?;
        builder
            .append_path_with_name(&path, rel)
            .map_err(|err| format!("写入备份文件失败: {}", err))?;
    }
    Ok(())
}

fn should_skip_path(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| SKIPPED_CONFIG_DIRS.contains(&name))
}

fn is_backup_archive_name(file_name: &str) -> bool {
    !file_name.ends_with('/') && file_name.ends_with(".tar.gz")
}

fn local_name(tag: &[u8]) -> &str {
    std::str::from_utf8(tag)
        .ok()
        .and_then(|name| name.rsplit(':').next())
        .unwrap_or("")
}

fn parse_webdav_modified_at(value: &str) -> Option<i64> {
    DateTime::parse_from_rfc2822(value)
        .ok()
        .map(|dt| dt.timestamp())
}

fn build_backup_entry(
    href: Option<String>,
    modified_at: Option<i64>,
    size_bytes: Option<u64>,
) -> Option<RemoteBackupEntry> {
    let href = href?;
    let file_name = href
        .rsplit('/')
        .find(|segment| !segment.is_empty())?
        .to_string();
    if !is_backup_archive_name(&file_name) {
        return None;
    }

    Some(RemoteBackupEntry {
        file_name,
        modified_at: modified_at.unwrap_or(0),
        size_bytes: size_bytes.unwrap_or(0),
    })
}

fn staged_restore_dir(config_dir: &Path) -> PathBuf {
    config_dir.join(RESTORE_STAGING_DIR)
}

fn staged_restore_archive_path(config_dir: &Path) -> PathBuf {
    staged_restore_dir(config_dir).join(STAGED_RESTORE_ARCHIVE)
}

fn staged_restore_meta_path(config_dir: &Path) -> PathBuf {
    staged_restore_dir(config_dir).join(STAGED_RESTORE_META)
}

fn read_staged_restore_meta(config_dir: &Path) -> Option<StagedRestoreMeta> {
    let bytes = fs::read(staged_restore_meta_path(config_dir)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

fn extract_restore_archive(archive_path: &Path, target_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(archive_path).map_err(|err| format!("打开恢复包失败: {}", err))?;
    let reader = BufReader::new(file);
    let decoder = GzDecoder::new(reader);
    let mut archive = tar::Archive::new(decoder);

    let entries = archive
        .entries()
        .map_err(|err| format!("读取恢复归档失败: {}", err))?;

    for entry in entries {
        let mut entry = entry.map_err(|err| format!("读取恢复条目失败: {}", err))?;
        let rel_path = sanitize_archive_path(
            &entry
                .path()
                .map_err(|err| format!("读取恢复路径失败: {}", err))?,
        )?;
        if rel_path.as_os_str().is_empty() {
            continue;
        }

        let output_path = target_dir.join(&rel_path);
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|err| format!("创建恢复目录失败: {}", err))?;
        }
        entry
            .unpack(&output_path)
            .map_err(|err| format!("解压恢复条目失败: {}", err))?;
    }

    Ok(())
}

fn sanitize_archive_path(path: &Path) -> Result<PathBuf, String> {
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Normal(value) => clean.push(value),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err("恢复包包含非法路径".to_string());
            }
        }
    }
    Ok(clean)
}

fn copy_restore_tree(src: &Path, dest: &Path) -> Result<usize, String> {
    let mut restored_files = 0;
    let entries = fs::read_dir(src).map_err(|err| format!("读取恢复目录失败: {}", err))?;
    for entry in entries {
        let entry = entry.map_err(|err| format!("读取恢复目录项失败: {}", err))?;
        let path = entry.path();
        let target = dest.join(entry.file_name());
        if path.is_dir() {
            fs::create_dir_all(&target).map_err(|err| format!("创建目标目录失败: {}", err))?;
            restored_files += copy_restore_tree(&path, &target)?;
        } else if path.is_file() {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|err| format!("创建目标父目录失败: {}", err))?;
            }
            fs::copy(&path, &target).map_err(|err| format!("写入恢复文件失败: {}", err))?;
            restored_files += 1;
        }
    }
    Ok(restored_files)
}

fn cleanup_restore_staging(config_dir: &Path) -> Result<(), String> {
    let staging_dir = staged_restore_dir(config_dir);
    if staging_dir.exists() {
        fs::remove_dir_all(staging_dir).map_err(|err| format!("清理恢复暂存目录失败: {}", err))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_remote_dir_defaults_to_onetcli_backup() {
        assert_eq!("onetcli-backup", normalize_remote_dir(""));
        assert_eq!("onetcli-backup", normalize_remote_dir(" / "));
    }

    #[test]
    fn normalize_remote_dir_trims_outer_slashes() {
        assert_eq!("apps/onetcli", normalize_remote_dir(" /apps/onetcli/ "));
    }

    #[test]
    fn build_backup_target_url_joins_endpoint_directory_and_file() {
        let url = build_backup_target_url(
            "https://dav.example.com/remote.php/dav/files/me/",
            "/apps/onetcli/",
            "onetcli-backup-20260513-102030.tar.gz",
        )
        .expect("url should be valid");

        assert_eq!(
            "https://dav.example.com/remote.php/dav/files/me/apps/onetcli/onetcli-backup-20260513-102030.tar.gz",
            url
        );
    }

    #[test]
    fn backup_filename_uses_timestamp_and_tar_gz_suffix() {
        assert_eq!(
            "onetcli-backup-20260513-102030.tar.gz",
            backup_filename_from_timestamp(1_778_667_630)
        );
    }

    #[test]
    fn parse_propfind_backups_extracts_archives_and_metadata() {
        let xml = r#"<?xml version="1.0"?>
<d:multistatus xmlns:d="DAV:">
  <d:response>
    <d:href>/dav/onetcli-backup/</d:href>
    <d:propstat>
      <d:prop>
        <d:getlastmodified>Wed, 13 May 2026 08:17:58 GMT</d:getlastmodified>
        <d:getcontentlength>0</d:getcontentlength>
      </d:prop>
    </d:propstat>
  </d:response>
  <d:response>
    <d:href>/dav/onetcli-backup/onetcli-backup-20260513-161758.tar.gz</d:href>
    <d:propstat>
      <d:prop>
        <d:getlastmodified>Wed, 13 May 2026 08:17:58 GMT</d:getlastmodified>
        <d:getcontentlength>2306867</d:getcontentlength>
      </d:prop>
    </d:propstat>
  </d:response>
  <d:response>
    <d:href>/dav/onetcli-backup/random.txt</d:href>
    <d:propstat>
      <d:prop>
        <d:getlastmodified>Wed, 13 May 2026 08:10:00 GMT</d:getlastmodified>
        <d:getcontentlength>12</d:getcontentlength>
      </d:prop>
    </d:propstat>
  </d:response>
</d:multistatus>"#;

        let entries = parse_propfind_backups(xml.as_bytes()).expect("xml should parse");

        assert_eq!(1, entries.len());
        assert_eq!(
            "onetcli-backup-20260513-161758.tar.gz",
            entries[0].file_name
        );
        assert_eq!(2_306_867, entries[0].size_bytes);
        assert_eq!(1_778_660_278, entries[0].modified_at);
    }

    #[test]
    fn backup_candidates_only_keep_supported_archive_names() {
        assert!(is_backup_archive_name(
            "onetcli-backup-20260513-161758.tar.gz"
        ));
        assert!(is_backup_archive_name("custom.snapshot.tar.gz"));
        assert!(!is_backup_archive_name("folder/"));
        assert!(!is_backup_archive_name("readme.txt"));
    }
}
