//! `insight-app`: the Insight window (ADR-0031, R2-13).
//!
//! The app owns the backend: one [`Server`] holds what's loaded, the last view and
//! the user's selection. The window calls it through Tauri commands, and
//! `insight mcp` reaches the same server over a local named pipe, so Claude and the
//! user act on one state (R2-09). `show` and `load` reach the window as events.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::sync::{Arc, Mutex};

use insight_mcp::Server;
use insight_mcp::bridge::{Endpoint, app_running, run_file};
use serde_json::Value;
use tauri::{Emitter, Manager, State};

type Shared = Arc<Mutex<Server>>;

fn lock(s: &Shared) -> std::sync::MutexGuard<'_, Server> {
    // A panic inside one call must not take the window down with it.
    s.lock().unwrap_or_else(|p| p.into_inner())
}

#[tauri::command]
async fn projects(s: State<'_, Shared>) -> Result<Value, String> {
    lock(&s).projects().map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn load(s: State<'_, Shared>, project: String, snapshot: Option<String>) -> Result<String, String> {
    lock(&s).load_main(&project, snapshot.as_deref()).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn graph(s: State<'_, Shared>) -> Result<Value, String> {
    lock(&s).graph().map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn file(s: State<'_, Shared>, path: String) -> Result<String, String> {
    lock(&s).file(&path).map_err(|e| format!("{e:#}"))
}

#[tauri::command]
async fn current_view(s: State<'_, Shared>) -> Result<Option<Value>, String> {
    Ok(lock(&s).view().cloned())
}

#[tauri::command]
async fn select(s: State<'_, Shared>, selection: Value) -> Result<(), String> {
    lock(&s).set_selection(selection);
    Ok(())
}

/// Serves `insight mcp` clients: the token line first, then one reply per request.
#[cfg(windows)]
async fn serve_pipe(ep: Endpoint, server: Shared) -> anyhow::Result<()> {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
    use tokio::net::windows::named_pipe::ServerOptions;
    let ep = Arc::new(ep);
    let mut first = true;
    loop {
        let pipe = ServerOptions::new().first_pipe_instance(first).reject_remote_clients(true).create(&ep.pipe)?;
        first = false;
        pipe.connect().await?;
        let (ep, server) = (ep.clone(), server.clone());
        tauri::async_runtime::spawn(async move {
            let (r, mut w) = tokio::io::split(pipe);
            let mut lines = BufReader::new(r).lines();
            match lines.next_line().await {
                Ok(Some(l)) if ep.accepts(&l) => {
                    if w.write_all(b"ok\n").await.is_err() {
                        return;
                    }
                }
                _ => return,
            }
            while let Ok(Some(line)) = lines.next_line().await {
                let server = server.clone();
                let reply = tauri::async_runtime::spawn_blocking(move || lock(&server).handle_line(&line)).await.ok().flatten().unwrap_or_default();
                if w.write_all(reply.as_bytes()).await.is_err() || w.write_all(b"\n").await.is_err() {
                    return;
                }
            }
        });
    }
}

fn main() {
    // One window owns the backend; a second launch leaves the first in charge.
    if app_running() {
        eprintln!("Insight is already running.");
        return;
    }
    let server: Shared = Arc::new(Mutex::new(Server::default()));
    tauri::Builder::default()
        .manage(server.clone())
        .invoke_handler(tauri::generate_handler![projects, load, graph, file, current_view, select])
        .setup(move |app| {
            let handle = app.handle().clone();
            lock(&server).attach_window(Box::new(move |event, payload| {
                let _ = handle.emit(event, payload);
            }));
            let ep = Endpoint::fresh()?;
            ep.publish()?;
            #[cfg(windows)]
            {
                let server = server.clone();
                tauri::async_runtime::spawn(async move {
                    if let Err(e) = serve_pipe(ep, server).await {
                        eprintln!("insight-app: the MCP pipe stopped: {e:#}");
                    }
                });
            }
            if let Some(w) = app.get_webview_window("main") {
                w.set_title(&format!("Insight{}", if cfg!(debug_assertions) { " (dev)" } else { "" }))?;
            }
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("the Insight window failed to start")
        .run(|_, e| {
            if let tauri::RunEvent::Exit = e {
                // Only our own run file: a newer app may have replaced it.
                if let Ok(f) = run_file()
                    && std::fs::read_to_string(&f).is_ok_and(|t| t.contains(&format!("\"pid\":{}", std::process::id())))
                {
                    let _ = std::fs::remove_file(f);
                }
            }
        });
}
