const COMMANDS: &[&str] = &[
    "call",
    "call_json",
    "fetch",
    "fetch_cancel",
    "fetch_send",
    "fetch_read_body",
];

fn main() {
    tauri_plugin::Builder::new(COMMANDS).build();
}
