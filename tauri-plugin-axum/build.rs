const COMMANDS: &[&str] = &["call"];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
