use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    ImportSources,
    CreateHomebrew,
    ArchiveSources,
    ManageUsers,
}

impl Permission {
    pub const ALL: [Permission; 4] = [
        Permission::ImportSources,
        Permission::CreateHomebrew,
        Permission::ArchiveSources,
        Permission::ManageUsers,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            Permission::ImportSources => "import_sources",
            Permission::CreateHomebrew => "create_homebrew",
            Permission::ArchiveSources => "archive_sources",
            Permission::ManageUsers => "manage_users",
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Permission::ImportSources => "Import Sources",
            Permission::CreateHomebrew => "Create Homebrew",
            Permission::ArchiveSources => "Archive Sources",
            Permission::ManageUsers => "Manage Users",
        }
    }
}

/// The full set of daisyUI built-in themes registered in `style/main.css` —
/// keep these two lists in sync. Themes are stored as plain strings (no enum)
/// since this is just a display preference, not something the backend
/// branches on; `change_theme` still validates against this list so only a
/// recognized theme name ever lands in `users.theme`.
pub const THEMES: [&str; 36] = [
    "guidance",
    "abyss",
    "acid",
    "aqua",
    "autumn",
    "black",
    "bumblebee",
    "business",
    "caramellatte",
    "cmyk",
    "coffee",
    "corporate",
    "cupcake",
    "cyberpunk",
    "dark",
    "dim",
    "dracula",
    "emerald",
    "fantasy",
    "forest",
    "garden",
    "halloween",
    "lemonade",
    "light",
    "lofi",
    "luxury",
    "night",
    "nord",
    "pastel",
    "retro",
    "silk",
    "sunset",
    "synthwave",
    "valentine",
    "winter",
    "wireframe",
];

/// The safe, client-facing view of a logged-in user — never carries
/// `password_hash`. This is both what `get_current_user` returns to the
/// browser and what server-side auth guards pass around internally.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: String,
    pub username: String,
    pub is_instance_owner: bool,
    pub permissions: Vec<Permission>,
    pub theme: String,
}

impl CurrentUser {
    /// is_instance_owner is a hard bootstrap escape hatch that can never be
    /// revoked; ManageUsers is a grantable permission that makes any other
    /// user a full-privilege admin too.
    pub fn is_admin(&self) -> bool {
        self.is_instance_owner || self.permissions.contains(&Permission::ManageUsers)
    }

    pub fn has_permission(&self, permission: Permission) -> bool {
        self.is_admin() || self.permissions.contains(&permission)
    }
}

/// Row shape for the admin user-management table — deliberately excludes
/// password_hash even though it's an admin-only view.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UserSummary {
    pub id: String,
    pub username: String,
    pub is_instance_owner: bool,
    pub permissions: Vec<Permission>,
}
