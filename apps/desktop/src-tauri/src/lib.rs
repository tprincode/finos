use std::path::PathBuf;
use std::sync::Arc;

use ai_gateway::GrokAdvisory;
use application_core::contracts::{CommandRequest, CommandResult, QueryRequest, QueryResult};
use application_core::ports::advisory::{Advisory, MissingKeyAdvisory};
use application_core::queries::{execute_command_on, execute_query_on};
use storage_sqlite::LocalPlatform;
use tauri::menu::{MenuBuilder, SubmenuBuilder};
use tauri::{AppHandle, Manager, State};

#[tauri::command]
async fn finance_query(
    platform: State<'_, Arc<LocalPlatform>>,
    request: QueryRequest,
) -> Result<QueryResult, String> {
    Ok(execute_query_on(platform.inner().as_ref(), platform.inner().as_ref(), request).await)
}

#[tauri::command]
async fn finance_command(
    platform: State<'_, Arc<LocalPlatform>>,
    mut request: CommandRequest,
) -> Result<CommandResult, String> {
    if matches!(
        request.command_name.as_str(),
        "PriceQuoteRetrieve" | "DeclarationRetrieve" | "MarketRetrieve"
    ) {
        let name = request.command_name.clone();
        let mut body: serde_json::Value = request
            .body_json
            .as_deref()
            .and_then(|raw| serde_json::from_str(raw).ok())
            .unwrap_or_else(|| serde_json::json!({}));
        let filled = tauri::async_runtime::spawn_blocking(move || {
            import_engine::enrich_retrieve_body(&name, &mut body);
            body
        })
        .await
        .map_err(|e| e.to_string())?;
        request.body_json = Some(filled.to_string());
    }
    Ok(execute_command_on(platform.inner().as_ref(), platform.inner().as_ref(), request).await)
}

#[tauri::command]
fn app_exit(app: AppHandle) {
    app.exit(0);
}

fn looks_like_sync_folder(path: &std::path::Path) -> bool {
    let s = path.to_string_lossy().to_lowercase();
    s.contains("onedrive") || s.contains("dropbox") || s.contains("icloud")
}

fn resolve_app_data_dir(preferred: PathBuf) -> PathBuf {
    if looks_like_sync_folder(&preferred) {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            return PathBuf::from(local).join("finos");
        }
    }
    preferred
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .setup(|app| {
            let preferred = app.path().app_local_data_dir()?;
            let dir = resolve_app_data_dir(preferred);
            std::fs::create_dir_all(&dir)?;
            let _ = dotenvy::from_filename(dir.join(".env"));
            let _ = dotenvy::dotenv();
            let advisory: Arc<dyn Advisory> = match GrokAdvisory::from_env() {
                Some(grok) => Arc::new(grok),
                None => Arc::new(MissingKeyAdvisory),
            };
            let platform = tauri::async_runtime::block_on(LocalPlatform::open_with_advisory(
                dir, advisory,
            ))
            .map_err(|e| e.to_string())?;
            app.manage(Arc::new(platform));
            let file_menu = SubmenuBuilder::new(app, "File")
                .quit_with_text("Exit")
                .build()?;
            let menu = MenuBuilder::new(app).item(&file_menu).build()?;
            app.set_menu(menu)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            finance_query,
            finance_command,
            app_exit
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
