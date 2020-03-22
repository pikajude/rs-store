#[derive(serde::Serialize)]
struct Builtins {
  store_path: String,
  local_state_path: String,
}

impl Default for Builtins {
  fn default() -> Self {
    Self {
      store_path: "/nix/store".into(),
      local_state_path: "/nix/var".into(),
    }
  }
}

fn main() {
  let mut builtins = Builtins::default();
  if let Ok(x) = std::env::var("NIX_STORE_DIR") {
    builtins.store_path = x;
  }
  if let Ok(x) = std::env::var("NIX_STATE_DIR") {
    builtins.local_state_path = x;
  }
  let dest = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("config.json");
  std::fs::write(dest, serde_json::to_string(&builtins).unwrap()).unwrap();
}
