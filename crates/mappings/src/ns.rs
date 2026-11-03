//! Namespace names used across Jarrow.
//!
//! These are names, not decisions: which namespace a loader runs on for a game version comes
//! from the loader profiles, never from this crate. Any other namespace name works as well.

/// Mojang's obfuscated names, as shipped in the vanilla jars (Fabric's `official` column).
pub const OBF: &str = "obf";

/// Fabric's intermediary names (`net/minecraft/class_2960`, `field_1234`, `method_5678`).
pub const INTERMEDIARY: &str = "intermediary";

/// Mojang's deobfuscated names from the ProGuard mapping files
/// (`net/minecraft/resources/ResourceLocation`).
pub const MOJANG: &str = "mojang";

/// Forge's SRG names from MCPConfig (`f_12345_`, `m_12345_`).
pub const SRG: &str = "srg";

/// Yarn's names (Fabric's `named` column).
pub const NAMED: &str = "named";
