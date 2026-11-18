use crate::error::ApiError;
use crate::AppState;
use akarbox_core::{detect_format, normalize_ext, pdf, Format, Options, Target, FORMATS};
use axum::body::Bytes;
use axum::extract::multipart::Field;
use axum::extract::{Multipart, Path, Query, State};
use axum::http::{header, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::{Deserialize, Serialize};
use std::path::{Path as FsPath, PathBuf};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

/// How many bytes of a file we look at to recognise its format.
const SNIFF_BYTES: usize = 4096;
/// Upper limit on PDFs in one merge request.
const MAX_MERGE_FILES: usize = 100;

#[derive(Serialize)]
pub struct Info {
    version: &'static str,
    converters: Vec<&'static str>,
    formats: &'static [Format],
    /// Whether PDF merge/split is available (needs qpdf).
    pdf_tools: bool,
}

pub async fn info(State(state): State<AppState>) -> Json<Info> {
    Json(Info {
        version: env!("CARGO_PKG_VERSION"),
        converters: state.registry.converter_ids(),
        formats: FORMATS,
        pdf_tools: state.pdf_tools,
    })
}

pub async fn targets(
    State(state): State<AppState>,
    Path(ext): Path<String>,
) -> Result<Json<Vec<Target>>, ApiError> {
    if Format::get(&ext).is_none() {
        return Err(akarbox_core::Error::UnknownFormat(ext).into());
    }
    Ok(Json(state.registry.targets(&ext)))
}

#[derive(Deserialize)]
pub struct DetectQuery {
    name: String,
}

#[derive(Serialize)]
pub struct Detected {
    format: &'static Format,
    targets: Vec<Target>,
}

/// `POST /api/detect?name=photo.jpg` with the first few KB of the file as the body.
///
/// The browser sends only the start of the file, so the UI can show the right
/// formats instantly without uploading the whole thing.
pub async fn detect(
    State(state): State<AppState>,
    Query(q): Query<DetectQuery>,
    body: Bytes,
) -> Result<Json<Detected>, ApiError> {
    let header = &body[..body.len().min(SNIFF_BYTES)];
    let format = detect_format(&q.name, header)
        .ok_or_else(|| akarbox_core::Error::UnknownFormat(q.name.clone()))?;
    Ok(Json(Detected {
        format,
        targets: state.registry.targets(format.ext),
    }))
}

/// `POST /api/convert` with multipart fields `target` (e.g. `"pdf"`), optional
/// `options` (JSON, see [`Options`]) and `file`.
pub async fn convert(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Response, ApiError> {
    // Every request gets its own temp directory, deleted when `dir` is dropped
    // at the end of this function, whether we succeed or fail.
    let dir = tempfile::tempdir()?;
    let mut target: Option<String> = None;
    let mut options = Options::default();
    let mut upload: Option<Upload> = None;

    while let Some(field) = multipart.next_field().await.map_err(bad_multipart)? {
        match field.name() {
            Some("target") => target = Some(field.text().await.map_err(bad_multipart)?),
            Some("options") => {
                options = Options::from_json(&field.text().await.map_err(bad_multipart)?)?
            }
            Some("file") => upload = Some(save_upload(field, dir.path(), 0).await?),
            _ => {} // ignore unknown fields
        }
    }

    let target =
        normalize_ext(&target.ok_or_else(|| ApiError::bad_request("missing `target` field"))?);
    let upload = upload.ok_or_else(|| ApiError::bad_request("missing `file` field"))?;
    let from = upload.format()?;
    let plan = state.registry.plan(from.ext, &target)?;

    // Wait for a free job slot.
    let _permit = state.jobs.acquire().await.expect("semaphore never closed");

    // Conversions block (they wait on external programs), so run them on tokio's
    // blocking thread pool. `move` hands ownership of the captured values to the closure.
    let registry = state.registry.clone();
    let work = dir.path().join("work");
    let input = upload.path.clone();
    let (bytes, ext) = blocking(move || {
        std::fs::create_dir_all(&work)?;
        let out = registry.execute(&plan, &input, &work, &options)?;
        Ok((std::fs::read(&out.path)?, out.ext))
    })
    .await?;

    Ok(download(bytes, &format!("{}.{ext}", upload.stem())))
}

/// `POST /api/pdf/merge`: multipart with several `file` fields, merged in order.
pub async fn pdf_merge(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Response, ApiError> {
    require_pdf_tools(&state)?;
    let dir = tempfile::tempdir()?;
    let mut uploads = Vec::new();
    while let Some(field) = multipart.next_field().await.map_err(bad_multipart)? {
        if field.name() == Some("file") {
            if uploads.len() == MAX_MERGE_FILES {
                return Err(ApiError::bad_request(format!(
                    "at most {MAX_MERGE_FILES} files can be merged at once"
                )));
            }
            let upload = save_upload(field, dir.path(), uploads.len()).await?;
            upload.expect_pdf()?;
            uploads.push(upload);
        }
    }
    if uploads.len() < 2 {
        return Err(ApiError::bad_request(
            "upload at least two PDF files to merge",
        ));
    }

    let _permit = state.jobs.acquire().await.expect("semaphore never closed");
    let inputs: Vec<PathBuf> = uploads.iter().map(|u| u.path.clone()).collect();
    let output = dir.path().join("merged.pdf");
    let bytes = blocking(move || {
        pdf::merge(&inputs, &output)?;
        Ok(std::fs::read(&output)?)
    })
    .await?;
    Ok(download(bytes, "merged.pdf"))
}

/// `POST /api/pdf/split`: multipart `file` + `pages` (e.g. `"1-3, 5"`).
/// Returns a PDF containing only those pages.
pub async fn pdf_split(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Response, ApiError> {
    require_pdf_tools(&state)?;
    let dir = tempfile::tempdir()?;
    let mut pages: Option<String> = None;
    let mut upload: Option<Upload> = None;
    while let Some(field) = multipart.next_field().await.map_err(bad_multipart)? {
        match field.name() {
            Some("pages") => pages = Some(field.text().await.map_err(bad_multipart)?),
            Some("file") => upload = Some(save_upload(field, dir.path(), 0).await?),
            _ => {}
        }
    }
    let pages = pages.ok_or_else(|| ApiError::bad_request("missing `pages` field"))?;
    let upload = upload.ok_or_else(|| ApiError::bad_request("missing `file` field"))?;
    upload.expect_pdf()?;

    let _permit = state.jobs.acquire().await.expect("semaphore never closed");
    let input = upload.path.clone();
    let output = dir.path().join("pages.pdf");
    let selection = pages.clone();
    let bytes = blocking(move || {
        pdf::extract_pages(&input, &selection, &output)?;
        Ok(std::fs::read(&output)?)
    })
    .await?;

    let label: String = pages
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == ',')
        .collect();
    Ok(download(
        bytes,
        &format!("{} (pages {label}).pdf", upload.stem()),
    ))
}

// ---------------------------------------------------------------------------
// Helpers shared by the handlers above.
// ---------------------------------------------------------------------------

/// A file received from the browser and saved to disk.
struct Upload {
    /// The name the user's file had, e.g. `"Holiday photo.HEIC"`.
    name: String,
    path: PathBuf,
    /// The first bytes of the file, used to detect its format.
    header: Vec<u8>,
}

impl Upload {
    fn format(&self) -> Result<&'static Format, ApiError> {
        detect_format(&self.name, &self.header)
            .ok_or_else(|| akarbox_core::Error::UnknownFormat(self.name.clone()).into())
    }

    fn expect_pdf(&self) -> Result<(), ApiError> {
        match self.format() {
            Ok(f) if f.ext == "pdf" => Ok(()),
            _ => Err(ApiError::bad_request(format!("{} is not a PDF", self.name))),
        }
    }

    /// File name without extension, made safe for a download header.
    fn stem(&self) -> String {
        let stem = FsPath::new(&self.name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        safe_filename(stem)
    }
}

/// Stream a multipart file field to disk chunk by chunk instead of holding it
/// in memory, then read back its first bytes.
async fn save_upload(mut field: Field<'_>, dir: &FsPath, index: usize) -> Result<Upload, ApiError> {
    let name = field.file_name().unwrap_or("upload").to_string();
    // The original name never touches the filesystem; we pick our own.
    let path = dir.join(format!("upload-{index}"));
    let mut file = tokio::fs::File::create(&path).await?;
    while let Some(chunk) = field.chunk().await.map_err(bad_multipart)? {
        file.write_all(&chunk).await?;
    }
    file.flush().await?;
    drop(file);

    let mut header = vec![0; SNIFF_BYTES];
    let n = tokio::fs::File::open(&path)
        .await?
        .read(&mut header)
        .await?;
    header.truncate(n);

    // Converters such as LibreOffice look at the extension, so give the file the
    // right one now that we know what it is.
    let mut upload = Upload { name, path, header };
    if let Ok(format) = upload.format() {
        let named = dir.join(format!("input-{index}.{}", format.ext));
        tokio::fs::rename(&upload.path, &named).await?;
        upload.path = named;
    }
    Ok(upload)
}

/// Run blocking work (external programs, file IO) on tokio's blocking pool.
///
/// This is a *generic* function: `T` is whatever the closure returns, and
/// `F: FnOnce() -> ... + Send + 'static` says "any closure that can be called
/// once and moved to another thread".
async fn blocking<T, F>(work: F) -> Result<T, ApiError>
where
    F: FnOnce() -> akarbox_core::Result<T> + Send + 'static,
    T: Send + 'static,
{
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|e| ApiError {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            message: format!("conversion task crashed: {e}"),
        })?
        .map_err(ApiError::from)
}

fn require_pdf_tools(state: &AppState) -> Result<(), ApiError> {
    if state.pdf_tools {
        Ok(())
    } else {
        Err(ApiError {
            status: StatusCode::NOT_IMPLEMENTED,
            message: "PDF tools need qpdf, which is not installed on this server".into(),
        })
    }
}

fn bad_multipart(e: axum::extract::multipart::MultipartError) -> ApiError {
    ApiError::bad_request(e.to_string())
}

/// Build a "please download this file" response.
fn download(bytes: Vec<u8>, file_name: &str) -> Response {
    let ext = FsPath::new(file_name)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let mime = mime_guess::from_ext(ext).first_or_octet_stream();
    (
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{file_name}\""),
            ),
        ],
        bytes,
    )
        .into_response()
}

/// Keep header-safe characters only, so a hostile file name can't break the response.
fn safe_filename(stem: &str) -> String {
    let cleaned: String = stem
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || " -_.()".contains(c) {
                c
            } else {
                '_'
            }
        })
        .take(100)
        .collect();
    if cleaned.trim().is_empty() {
        "converted".into()
    } else {
        cleaned
    }
}

#[cfg(test)]
mod tests {
    use super::safe_filename;

    #[test]
    fn sanitizes_filenames() {
        assert_eq!(safe_filename("report \"final\"\r\n"), "report _final___");
        assert_eq!(safe_filename("résumé"), "r_sum_");
        assert_eq!(safe_filename(""), "converted");
    }
}
