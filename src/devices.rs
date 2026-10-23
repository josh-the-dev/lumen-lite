//! Devices lumen-lite knows how to talk to.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Keyboard,
    Mouse,
}

#[derive(Debug)]
pub struct Model {
    pub pid: u16,
    pub name: &'static str,
    pub kind: Kind,
    /// Number of independently colored lighting zones.
    pub zones: usize,
}

pub const MODELS: &[Model] = &[
    Model {
        pid: 0x1BA4,
        name: "K55 RGB PRO",
        kind: Kind::Keyboard,
        zones: 6,
    },
    Model {
        pid: 0x1BE3,
        name: "Scimitar RGB Elite",
        kind: Kind::Mouse,
        zones: 5,
    },
];

#[must_use]
pub fn by_pid(pid: u16) -> Option<&'static Model> {
    MODELS.iter().find(|model| model.pid == pid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pids_are_unique() {
        for (i, a) in MODELS.iter().enumerate() {
            assert!(MODELS[i + 1..].iter().all(|b| b.pid != a.pid), "{}", a.name);
        }
    }

    #[test]
    fn looks_up_by_pid() {
        assert_eq!(by_pid(0x1BA4).map(|m| m.name), Some("K55 RGB PRO"));
        assert!(by_pid(0x0000).is_none());
    }
}
