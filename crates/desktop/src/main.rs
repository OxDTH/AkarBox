//! AkarBox desktop app.
//!
//! Instead of re-implementing the API with Tauri commands, the desktop app starts
//! the very same axum server used for self-hosting, bound to `127.0.0.1` on a
//! random free port, and points a native window at it. One frontend, one API, two
//! ways to ship it.

// Hide the extra console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::webview::DownloadEvent;
use tauri::{Manager, WebviewUrl, WebviewWindowBuilder};

fn main() {
    tracing_subscriber::fmt().init();

    // Tauri owns the main thread for the GUI, so the web server gets its own tokio
    // runtime on a background thread. A channel sends the chosen port back to us.
    let (port_tx, port_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let rt = tokio::runtime::Runtime::new().expect("tokio runtime");
        rt.block_on(async move {
            // Port 0 = "let the OS pick a free port". Only localhost can reach it.
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
                .await
                .expect("bind");
            port_tx.send(listener.local_addr().unwrap().port()).unwrap();

            let mut config = akarbox_server::Config::from_env().expect("config");
            config.addr = "127.0.0.1:0".into();
            akarbox_server::serve(listener, config)
                .await
                .expect("server crashed");
        });
    });
    let port: u16 = port_rx.recv().expect("server failed to start");
    let url = format!("http://127.0.0.1:{port}/");

    tauri::Builder::default()
        .setup(move |app| {
            let downloads = app.path().download_dir().ok();
            WebviewWindowBuilder::new(app, "main", WebviewUrl::External(url.parse()?))
                .title("AkarBox")
                .inner_size(900.0, 760.0)
                .min_inner_size(420.0, 500.0)
                // Save converted files straight to the user's Downloads folder.
                .on_download(move |_webview, event| {
                    if let DownloadEvent::Requested { destination, .. } = event {
                        if let (Some(dir), Some(name)) = (&downloads, destination.file_name()) {
                            *destination = dir.join(name);
                        }
                    }
                    true // allow the download
                })
                .build()?;
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running AkarBox");
}
