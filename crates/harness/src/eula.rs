//! The Minecraft EULA gate (decision D-013, amendment A-6).
//!
//! A dedicated server only runs after the Minecraft EULA has been accepted. That acceptance
//! belongs to the project owner, so the harness never gives it: it reads it from the
//! environment and, when it is absent, starts nothing and writes no `eula.txt`.

/// The environment variables the gate reads, in order.
pub const EULA_VARIABLES: [&str; 2] = ["MINECRAFT_EULA_ACCEPTED", "MINECRAFT_EULA"];

/// Values that mean "accepted", compared case-insensitively after trimming.
pub const ACCEPTED_VALUES: [&str; 4] = ["true", "yes", "1", "accepted"];

/// Whether the owner's EULA acceptance is present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EulaGate {
    open_by: Option<&'static str>,
}

impl EulaGate {
    /// Reads the gate from the process environment.
    pub fn from_env() -> Self {
        Self::from_lookup(|name| std::env::var(name).ok())
    }

    /// Reads the gate through `lookup`, which maps a variable name to its value. Tests use
    /// this instead of mutating the process environment.
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Self {
        let open_by = EULA_VARIABLES.into_iter().find(|name| {
            lookup(name).is_some_and(|value| {
                let value = value.trim();
                ACCEPTED_VALUES
                    .iter()
                    .any(|accepted| accepted.eq_ignore_ascii_case(value))
            })
        });
        Self { open_by }
    }

    /// A gate that is closed. Nothing can start through it.
    pub const fn closed() -> Self {
        Self { open_by: None }
    }

    /// Whether servers may start.
    pub const fn is_open(&self) -> bool {
        self.open_by.is_some()
    }

    /// The variable that opened the gate, if any (never its value).
    pub const fn opened_by(&self) -> Option<&'static str> {
        self.open_by
    }

    /// The line explaining why nothing ran, for logs and run records.
    pub const fn closed_message() -> &'static str {
        "not run: the Minecraft EULA has not been accepted (set MINECRAFT_EULA_ACCEPTED=true \
         only if you have read and accepted https://www.minecraft.net/en-us/eula)"
    }

    /// The `eula.txt` contents, available only when the gate is open.
    pub fn eula_txt(&self) -> Option<String> {
        self.open_by.map(|name| {
            format!(
                "# Written by jarrow-harness because {name} says the Minecraft EULA was accepted.\n\
                 # https://www.minecraft.net/en-us/eula\n\
                 eula=true\n"
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gate(vars: &[(&str, &str)]) -> EulaGate {
        let vars: Vec<(String, String)> = vars
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        EulaGate::from_lookup(move |name| {
            vars.iter().find(|(k, _)| k == name).map(|(_, v)| v.clone())
        })
    }

    #[test]
    fn accepted_values_open_the_gate_case_insensitively() {
        for value in ["true", "TRUE", " yes ", "Yes", "1", "accepted", "ACCEPTED"] {
            for name in EULA_VARIABLES {
                let gate = gate(&[(name, value)]);
                assert!(gate.is_open(), "{name}={value:?}");
                assert_eq!(gate.opened_by(), Some(name));
                assert!(gate.eula_txt().unwrap().contains("\neula=true\n"));
            }
        }
    }

    #[test]
    fn anything_else_keeps_it_closed() {
        for value in [
            "",
            "false",
            "no",
            "0",
            "y",
            "accept",
            "truee",
            "on",
            "eula=true",
        ] {
            let gate = gate(&[
                ("MINECRAFT_EULA_ACCEPTED", value),
                ("MINECRAFT_EULA", value),
            ]);
            assert!(!gate.is_open(), "{value:?}");
            assert_eq!(gate.eula_txt(), None);
        }
        assert!(!gate(&[]).is_open());
        assert!(!gate(&[("EULA", "true")]).is_open());
        assert!(!EulaGate::closed().is_open());
    }

    #[test]
    fn either_variable_is_enough() {
        let gate = gate(&[
            ("MINECRAFT_EULA_ACCEPTED", "no"),
            ("MINECRAFT_EULA", "true"),
        ]);
        assert_eq!(gate.opened_by(), Some("MINECRAFT_EULA"));
    }

    #[test]
    fn the_closed_message_names_the_variable_and_the_eula() {
        let message = EulaGate::closed_message();
        assert!(message.contains("MINECRAFT_EULA_ACCEPTED"));
        assert!(message.contains("https://www.minecraft.net/en-us/eula"));
    }
}
