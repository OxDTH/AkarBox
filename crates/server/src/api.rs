use crate::error::ApiError;
use crate::AppState;
use akarbox_core::{detect_format, normalize_ext, Format, Target, FORMATS};
use axum::extract::{Multipart, Path, State};
use axum::http::header;
use axum::response::{IntoResponse, Response};
use axum::Json;
use serde::Serialize;
use tokio::io::AsyncWriteExt;

#[derive(Serialize)]
pub struct Info {
    version: &'static str,
    converters: Vec<&'static str>,
    formats: &'static [Format],
}

pub async fn info(State(state): State<AppState>) -> Json<Info> {
    Json(Info {
        version: env!("CARGO_PKG_VERSION"),
        converters: state.registry.converter_ids(),
        formats: FORMATS,
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

/// `POST /api/convert` with multipart fields `target` (e.g. `"pdf"`) and `file`.
pub async fn convert(
    State(state): State<AppState>,
    mut multipart: Multipart,
) -> Result<Response, ApiError> {
    // Every request gets its own temp directory, deleted when `dir` is dropped
    // at the end of this function, whether we succeed or fail.
    let dir = tempfile::tempdir()?;
    let mut target: Option<String> = None;
    let mut upload: Option<(String, std::path::PathBuf)> = None; // (original name, path)

    let bad = |e: axum::extract::multipart::MultipartError| ApiError::bad_request(e.to_string());
    while let Some(mut field) = multipart.next_field().await.map_err(bad)? {
        match field.name() {
            Some("target") => target = Some(field.text().await.map_err(bad)?),
            Some("file") => {
                let name = field.file_name().unwrap_or("upload").to_string();
                let format = detect_format(&name)
                    .ok_or_else(|| akarbox_core::Error::UnknownFormat(name.clone()))?;
                let path = dir.path().join(format!("input.{}", format.ext));
                // Stream the upload to disk chunk by chunk instead of holding it in memory.
                let mut file = tokio::fs::File::create(&path).await?;
                while let Some(chunk) = field.chunk().await.map_err(bad)? {
                    file.write_all(&chunk).await?;
                }
                file.flush().await?;
                upload = Some((name, path));
            }
            _ => {} // ignore unknown fields
        }
    }

    let target =
        normalize_ext(&target.ok_or_else(|| ApiError::bad_request("missing `target` field"))?);
    let (name, input) = upload.ok_or_else(|| ApiError::bad_request("missing `file` field"))?;
    let from = detect_format(&name).expect("checked above").ext;
    let plan = state.registry.plan(from, &target)?;

    // Wait for a free job slot.
    let _permit = state.jobs.acquire().await.expect("semaphore never closed");

    // Conversions block (they wait on external programs), so run them on tokio's
    // blocking thread pool. `move` hands ownership of the captured values to the closure.
    let registry = state.registry.clone();
    let work = dir.path().join("work");
    let output = tokio::task::spawn_blocking(move || -> akarbox_core::Result<Vec<u8>> {
        std::fs::create_dir_all(&work)?;
        let out = registry.execute(&plan, &input, &work)?;
        Ok(std::fs::read(out)?)
    })
    .await
    .map_err(|e| ApiError {
        status: axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        message: format!("conversion task crashed: {e}"),
    })??; // first `?` = did the thread crash, second `?` = did the conversion fail

    let stem = std::path::Path::new(&name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("converted");
    let download = format!("{}.{target}", safe_filename(stem));
    let mime = mime_guess::from_ext(&target).first_or_octet_stream();

    Ok((
        [
            (header::CONTENT_TYPE, mime.to_string()),
            (
                header::CONTENT_DISPOSITION,
                format!("attachment; filename=\"{download}\""),
            ),
        ],
        output,
    )
        .into_response())
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
