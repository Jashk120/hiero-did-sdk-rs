#[derive(Clone, Debug)]
pub enum VaultAuth {
    Token(String),
    AppRole { role_id: String, secret_id: String },
}
#[derive(Clone, Debug)]
pub struct VaultSignerConfig {
    pub vault_url: String,
    pub auth: VaultAuth,
    pub key_name: String,
    pub mount_path: String,
}

impl VaultSignerConfig {
    pub fn new(vault_url: impl Into<String>, auth: VaultAuth, key_name: impl Into<String>) -> Self {
        Self {
            vault_url: vault_url.into(),
            auth,
            key_name: key_name.into(),
            mount_path: "transit".into(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        VaultAuth,
        VaultSignerConfig,
    };

    #[test]
    fn new_uses_transit_mount_by_default() {
        let cfg = VaultSignerConfig::new(
            "http://127.0.0.1:8200",
            VaultAuth::Token("token".into()),
            "did-key",
        );

        assert_eq!(cfg.vault_url, "http://127.0.0.1:8200");
        assert_eq!(cfg.key_name, "did-key");
        assert_eq!(cfg.mount_path, "transit");
        assert!(matches!(cfg.auth, VaultAuth::Token(ref token) if token == "token"));
    }
}
