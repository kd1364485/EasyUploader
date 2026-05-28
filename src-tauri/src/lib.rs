use std::fs;
use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::sync::Mutex;
use ignore::Walk;
use serde::{Deserialize, Serialize};
use once_cell::sync::Lazy;
use rayon::prelude::*;

#[derive(Deserialize, Serialize)]
struct ExtensionsConfig {
    text_extensions: Vec<String>,
}

// 拡張子リストのキャッシュ
static EXTENSIONS_CACHE: Lazy<Mutex<Vec<String>>> = Lazy::new(|| {
    Mutex::new(load_extensions_from_file())
});

fn load_extensions_from_file() -> Vec<String> {
    let config_path = PathBuf::from("extensions.json");
    
    if let Ok(content) = fs::read_to_string(&config_path) {
        if let Ok(config) = serde_json::from_str::<ExtensionsConfig>(&content) {
            return config.text_extensions;
        }
    }
    
    // 設定ファイルの読み込みに失敗した場合のデフォルト値
    vec![
        "js", "ts", "jsx", "tsx", "py", "rs", "go", "java", "c", "cpp", "h", "hpp",
        "cs", "php", "rb", "swift", "kt", "scala", "clj", "hs", "ml", "fs",
        "md", "txt", "json", "yaml", "yml", "toml", "xml", "html", "css", "scss",
        "sql", "sh", "bash", "zsh", "ps1", "bat", "cmd", "dockerfile", "makefile",
        "r", "m", "lua", "pl", "pm", "t", "vue", "svelte", "astro"
    ].iter().map(|s| s.to_string()).collect()
}

fn get_extensions_cached() -> Vec<String> {
    EXTENSIONS_CACHE.lock().unwrap().clone()
}

#[tauri::command]
fn get_extensions() -> Vec<String> {
    get_extensions_cached()
}

#[tauri::command]
fn save_extensions(extensions: Vec<String>) -> Result<(), String> {
    let config = ExtensionsConfig {
        text_extensions: extensions.clone(),
    };
    
    let json_content = serde_json::to_string_pretty(&config)
        .map_err(|e| format!("JSONシリアライズエラー: {}", e))?;
    
    fs::write("extensions.json", json_content)
        .map_err(|e| format!("ファイル書き込みエラー: {}", e))?;
    
    // キャッシュを更新
    *EXTENSIONS_CACHE.lock().unwrap() = extensions;
    
    Ok(())
}

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

fn is_text_file(path: &Path) -> bool {
    let extensions = get_extensions_cached();
    
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
        .unwrap_or(false)
}

fn should_exclude_directory(dir_name: &str) -> bool {
    const EXCLUDED_DIRS: &[&str] = &[
        "node_modules", ".git", ".svn", ".hg", "dist", "build", "target",
        "out", "bin", "obj", "vendor", ".vscode", ".idea", "coverage",
        "__pycache__", ".pytest_cache", ".next", ".nuxt", ".cache",
    ];
    
    EXCLUDED_DIRS.contains(&dir_name)
}

fn read_file_safely(path: &Path) -> Result<String, String> {
    use std::io::Read;
    
    let mut file = fs::File::open(path)
        .map_err(|e| format!("ファイルオープンエラー: {}", e))?;
    
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)
        .map_err(|e| format!("ファイル読み込みエラー: {}", e))?;
    
    // UTF-8検証
    std::str::from_utf8(&buffer)
        .map_err(|_| "ファイルはUTF-8エンコーディングではありません（バイナリファイルの可能性があります）".to_string())?;
    
    // UTF-8文字列に変換
    String::from_utf8(buffer)
        .map_err(|e| format!("UTF-8変換エラー: {}", e))
}

// ファイル情報を保持する構造体
#[derive(Clone)]
struct FileInfo {
    relative_path: String,
    full_path: PathBuf,
    extension: String,
}

// ディレクトリを1回だけ走査してファイル情報を収集
fn collect_files(source_path: &Path) -> Result<Vec<FileInfo>, String> {
    let mut files: Vec<FileInfo> = Vec::new();
    let walker = Walk::new(source_path);
    
    for result in walker {
        let entry = result.map_err(|e| format!("ディレクトリ走査エラー: {}", e))?;
        let entry_path = entry.path().to_path_buf();
        
        // ディレクトリの除外チェック
        if entry_path.is_dir() {
            if let Some(dir_name) = entry_path.file_name().and_then(|n| n.to_str()) {
                if should_exclude_directory(dir_name) {
                    continue;
                }
            }
            continue;
        }
        
        // テキストファイルのみを対象にする
        if !is_text_file(&entry_path) {
            continue;
        }
        
        let relative_path = entry_path
            .strip_prefix(source_path)
            .map_err(|e| format!("相対パス計算エラー: {}", e))?;
        
        let path_str = relative_path
            .to_str()
            .ok_or("パス文字列変換エラー")?;
        
        // Windowsのパス区切り文字をスラッシュに変換
        let normalized = path_str.replace('\\', "/");
        
        let extension = entry_path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("")
            .to_lowercase();
        
        files.push(FileInfo {
            relative_path: normalized,
            full_path: entry_path,
            extension,
        });
    }
    
    files.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    
    Ok(files)
}

fn generate_file_tree(files: &[FileInfo]) -> Result<String, String> {
    println!("収集したファイル数: {}", files.len());
    
    // ツリー文字列を生成
    let mut output = String::from("■ Project Structure\n\n");
    
    // シンプルなツリー出力
    let mut last_dir: Option<String> = None;
    
    for file in files {
        let parts: Vec<&str> = file.relative_path.split('/').collect();
        
        if parts.len() > 1 {
            let dir = parts[..parts.len()-1].join("/");
            let filename = parts.last().unwrap_or(&"");
            
            // ディレクトリが変わったら出力
            if last_dir.as_ref() != Some(&dir) {
                output.push_str(&format!("{}/\n", dir));
                last_dir = Some(dir.clone());
            }
            
            output.push_str(&format!("  {}\n", filename));
        } else {
            // ルートのファイル
            output.push_str(&format!("{}\n", file.relative_path));
        }
    }
    
    Ok(output)
}

fn generate_project_overview(source_path: &Path, files: &[FileInfo]) -> Result<String, String> {
    let mut tech_stack: Vec<String> = Vec::new();
    let mut file_counts: HashMap<String, usize> = HashMap::new();
    
    for file in files {
        *file_counts.entry(file.extension.clone()).or_insert(0) += 1;
    }
    
    // テックスタックを推測
    if file_counts.contains_key(&"js".to_string()) || file_counts.contains_key(&"jsx".to_string()) {
        tech_stack.push("JavaScript".to_string());
    }
    if file_counts.contains_key(&"ts".to_string()) || file_counts.contains_key(&"tsx".to_string()) {
        tech_stack.push("TypeScript".to_string());
    }
    if file_counts.contains_key(&"py".to_string()) {
        tech_stack.push("Python".to_string());
    }
    if file_counts.contains_key(&"rs".to_string()) {
        tech_stack.push("Rust".to_string());
    }
    if file_counts.contains_key(&"go".to_string()) {
        tech_stack.push("Go".to_string());
    }
    if file_counts.contains_key(&"java".to_string()) {
        tech_stack.push("Java".to_string());
    }
    if file_counts.contains_key(&"cpp".to_string()) || file_counts.contains_key(&"c".to_string()) {
        tech_stack.push("C/C++".to_string());
    }
    if file_counts.contains_key(&"html".to_string()) || file_counts.contains_key(&"css".to_string()) {
        tech_stack.push("HTML/CSS".to_string());
    }
    if file_counts.contains_key(&"json".to_string()) {
        tech_stack.push("JSON configuration".to_string());
    }
    if file_counts.contains_key(&"md".to_string()) {
        tech_stack.push("Markdown documentation".to_string());
    }
    
    let folder_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("Project");
    
    let mut overview = String::from("■ Project Overview\n\n");
    overview.push_str(&format!("**Project Name**: {}\n\n", folder_name));
    overview.push_str("**Tech Stack**:\n");
    
    if tech_stack.is_empty() {
        overview.push_str("- Various text files\n");
    } else {
        for tech in &tech_stack {
            overview.push_str(&format!("- {}\n", tech));
        }
    }
    
    overview.push_str("\n**File Statistics**:\n");
    for (ext, count) in &file_counts {
        overview.push_str(&format!("- .{}: {} files\n", ext, count));
    }
    
    overview.push_str("\n**Purpose**: This archive contains source code files for AI analysis.\n");
    overview.push_str("The files have been filtered to include only text-based source code files.\n");
    overview.push_str("Binary files and common build artifacts have been excluded.\n");
    
    Ok(overview)
}

#[tauri::command]
fn generate_markdown(path: &str) -> Result<String, String> {
    let source_path = Path::new(path);
    
    if !source_path.exists() {
        return Err("指定されたパスが存在しません".to_string());
    }
    
    let folder_name = source_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("archive");
    
    // EasyUploaderディレクトリを作成（既存の場合は削除して再作成）
    let output_dir = source_path.join("EasyUploader");
    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)
            .map_err(|e| format!("EasyUploaderディレクトリ削除エラー: {}", e))?;
    }
    fs::create_dir_all(&output_dir)
        .map_err(|e| format!("EasyUploaderディレクトリ作成エラー: {}", e))?;
    
    // ファイルを1回だけ収集
    let files = collect_files(source_path)?;
    
    // README（プロジェクト概要とファイルツリー）を生成
    let file_tree = generate_file_tree(&files)?;
    let project_overview = generate_project_overview(source_path, &files)?;
    
    // 全体のREADMEファイルを作成
    let readme_filename = format!("{}_README.txt", folder_name);
    let readme_path = output_dir.join(&readme_filename);
    
    let mut readme_content = String::new();
    readme_content.push_str("■ AI Analysis Archive\n\n");
    readme_content.push_str(&project_overview);
    readme_content.push_str("\n\n");
    readme_content.push_str(&file_tree);
    readme_content.push_str("\n\n---\n\n");
    readme_content.push_str("■ Split Files\n\n");
    readme_content.push_str("The source code has been split into multiple files for easier AI analysis.\n\n");
    
    // ディレクトリごとにファイルをグループ化
    let mut dir_groups: std::collections::HashMap<String, Vec<FileInfo>> = std::collections::HashMap::new();
    let mut root_files: Vec<FileInfo> = Vec::new();
    
    for file in &files {
        let parts: Vec<&str> = file.relative_path.split('/').collect();
        
        if parts.len() > 1 {
            // ディレクトリ内のファイル
            let top_dir = parts[0];
            dir_groups.entry(top_dir.to_string())
                .or_insert_with(Vec::new)
                .push(file.clone());
        } else {
            // ルートのファイル
            root_files.push(file.clone());
        }
    }
    
    // ルートファイル用のMarkdownを作成（行数制限で分割）
    let mut total_output_files: usize = 0;
    if !root_files.is_empty() {
        // 並列でファイルを読み込み
        let root_file_contents: Vec<(String, Result<String, String>)> = root_files
            .par_iter()
            .map(|file| {
                (file.relative_path.clone(), read_file_safely(&file.full_path))
            })
            .collect();
        
        let mut root_file_index = 1;
        let mut current_lines = 0;
        const MAX_LINES_PER_FILE: usize = 5000; // 5000行ごとに分割
        
        let mut root_content = String::new();
        root_content.push_str("■ Root Files\n\n");
        
        for (relative_path, content_result) in root_file_contents {
            let content = match content_result {
                Ok(c) => c,
                Err(e) => {
                    println!("スキップ: {} ({})", relative_path, e);
                    continue;
                }
            };
            
            let file_lines = content.lines().count();
            
            // ファイル追加で制限を超える場合は現在のファイルを保存して新規作成
            if current_lines + file_lines > MAX_LINES_PER_FILE && current_lines > 0 {
                root_content.push_str("\n---\n\nGenerated by EasyUploader");
                
                let root_filename = format!("{}_Root_{}.txt", folder_name, root_file_index);
                let root_path = output_dir.join(&root_filename);
                
                fs::write(&root_path, root_content)
                    .map_err(|e| format!("Root Markdownファイル書き込みエラー: {}", e))?;
                
                readme_content.push_str(&format!("- **Root files part {}**: {}\n", root_file_index, root_filename));
                
                total_output_files += 1;
                root_file_index += 1;
                current_lines = 0;
                root_content = String::new();
                root_content.push_str("■ Root Files\n\n");
            }
            
            root_content.push_str(&format!("■ File: {}\n\n", relative_path));
            root_content.push_str(&content);
            root_content.push_str("\n\n");
            
            current_lines += file_lines;
        }
        
        // 残りの内容を保存
        if current_lines > 0 {
            root_content.push_str("\n---\n\nGenerated by EasyUploader");
            
            let root_filename = format!("{}_Root_{}.txt", folder_name, root_file_index);
            let root_path = output_dir.join(&root_filename);
            
            fs::write(&root_path, root_content)
                .map_err(|e| format!("Root Markdownファイル書き込みエラー: {}", e))?;
            
            readme_content.push_str(&format!("- **Root files part {}**: {}\n", root_file_index, root_filename));
            
            total_output_files += 1;
        }
    }
    
    // ディレクトリごとのMarkdownを作成（行数制限で分割）
    let mut dir_names: Vec<_> = dir_groups.keys().cloned().collect();
    dir_names.sort();
    
    for dir_name in &dir_names {
        if let Some(files) = dir_groups.get(dir_name) {
            // 並列でファイルを読み込み
            let dir_file_contents: Vec<(String, Result<String, String>)> = files
                .par_iter()
                .map(|file| {
                    (file.relative_path.clone(), read_file_safely(&file.full_path))
                })
                .collect();
            
            let mut dir_file_index = 1;
            let mut current_lines = 0;
            const MAX_LINES_PER_FILE: usize = 5000; // 5000行ごとに分割
            
            let mut dir_content = String::new();
            dir_content.push_str(&format!("■ Directory: {}\n\n", dir_name));
            
            for (relative_path, content_result) in dir_file_contents {
                let content = match content_result {
                    Ok(c) => c,
                    Err(e) => {
                        println!("スキップ: {} ({})", relative_path, e);
                        continue;
                    }
                };
                
                let file_lines = content.lines().count();
                
                // ファイル追加で制限を超える場合は現在のファイルを保存して新規作成
                if current_lines + file_lines > MAX_LINES_PER_FILE && current_lines > 0 {
                    dir_content.push_str("\n---\n\nGenerated by EasyUploader");
                    
                    let dir_filename = format!("{}_{}_{}.txt", folder_name, dir_name, dir_file_index);
                    let dir_path = output_dir.join(&dir_filename);
                    
                    fs::write(&dir_path, dir_content)
                        .map_err(|e| format!("Directory Markdownファイル書き込みエラー: {}", e))?;
                    
                    readme_content.push_str(&format!("- **{} part {}**: {}\n", dir_name, dir_file_index, dir_filename));
                    
                    total_output_files += 1;
                    dir_file_index += 1;
                    current_lines = 0;
                    dir_content = String::new();
                    dir_content.push_str(&format!("■ Directory: {}\n\n", dir_name));
                }
                
                dir_content.push_str(&format!("■ File: {}\n\n", relative_path));
                dir_content.push_str(&content);
                dir_content.push_str("\n\n");
                
                current_lines += file_lines;
            }
            
            // 残りの内容を保存
            if current_lines > 0 {
                dir_content.push_str("\n---\n\nGenerated by EasyUploader");
                
                let dir_filename = format!("{}_{}_{}.txt", folder_name, dir_name, dir_file_index);
                let dir_path = output_dir.join(&dir_filename);
                
                fs::write(&dir_path, dir_content)
                    .map_err(|e| format!("Directory Markdownファイル書き込みエラー: {}", e))?;
                
                readme_content.push_str(&format!("- **{} part {}**: {}\n", dir_name, dir_file_index, dir_filename));
                
                total_output_files += 1;
            }
        }
    }
    
    readme_content.push_str("\n---\n\nGenerated by EasyUploader");
    
    // READMEファイルを書き込み
    fs::write(&readme_path, readme_content)
        .map_err(|e| format!("READMEファイル書き込みエラー: {}", e))?;
    
    let total_files = root_files.len() + dir_groups.values().map(|v| v.len()).sum::<usize>();
    println!("{}個のファイルを{}個のテキストファイルに分割しました", total_files, total_output_files);
    
    Ok(readme_path.to_string_lossy().to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![greet, generate_markdown, get_extensions, save_extensions])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
