use std::fmt;
use std::str::FromStr;

/// A typed format version for spec files (e.g., "1.0").
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FormatVersion {
    pub major: u16,
    pub minor: u16,
}

impl FormatVersion {
    pub const CURRENT: FormatVersion = FormatVersion { major: 1, minor: 0 };

    pub fn new(major: u16, minor: u16) -> Self {
        Self { major, minor }
    }
}

impl fmt::Display for FormatVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl FromStr for FormatVersion {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 {
            return Err(format!("invalid format version: expected 'X.Y', got '{s}'"));
        }
        let major = parts[0]
            .parse::<u16>()
            .map_err(|_| format!("invalid major version: '{}'", parts[0]))?;
        let minor = parts[1]
            .parse::<u16>()
            .map_err(|_| format!("invalid minor version: '{}'", parts[1]))?;
        Ok(FormatVersion { major, minor })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_version_display() {
        assert_eq!(FormatVersion::new(1, 0).to_string(), "1.0");
        assert_eq!(FormatVersion::new(2, 3).to_string(), "2.3");
    }

    #[test]
    fn format_version_parse() {
        assert_eq!("1.0".parse::<FormatVersion>(), Ok(FormatVersion::new(1, 0)));
        assert_eq!("2.3".parse::<FormatVersion>(), Ok(FormatVersion::new(2, 3)));
        assert!("bad".parse::<FormatVersion>().is_err());
        assert!("1.2.3".parse::<FormatVersion>().is_err());
    }

    #[test]
    fn format_version_ordering() {
        let v1_0 = FormatVersion::new(1, 0);
        let v1_1 = FormatVersion::new(1, 1);
        let v2_0 = FormatVersion::new(2, 0);
        assert!(v1_0 < v1_1);
        assert!(v1_1 < v2_0);
        assert!(v1_0 < v2_0);
    }
}
