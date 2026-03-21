//! Version management and compatibility checking.
//!
//! This module provides types for managing backend versions and checking compatibility.

use semver::{Version, VersionReq};
use std::fmt;

/// Backend version information.
///
/// Uses semantic versioning (`SemVer`) for version management. Backends with the
/// same major version are considered compatible.
///
/// # Examples
///
/// ```
/// use canlink_hal::BackendVersion;
///
/// let version = BackendVersion::new(1, 2, 3);
/// assert_eq!(version.major(), 1);
/// assert_eq!(version.minor(), 2);
/// assert_eq!(version.patch(), 3);
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BackendVersion {
    version: Version,
}

impl BackendVersion {
    /// Create a new backend version.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendVersion;
    ///
    /// let version = BackendVersion::new(1, 2, 3);
    /// assert_eq!(version.to_string(), "1.2.3");
    /// ```
    #[must_use]
    pub fn new(major: u64, minor: u64, patch: u64) -> Self {
        Self {
            version: Version::new(major, minor, patch),
        }
    }

    /// Parse a version string.
    ///
    /// # Errors
    ///
    /// Returns an error if the version string is invalid.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendVersion;
    ///
    /// let version = BackendVersion::parse("1.2.3").unwrap();
    /// assert_eq!(version.major(), 1);
    /// ```
    pub fn parse(s: &str) -> Result<Self, semver::Error> {
        Ok(Self {
            version: Version::parse(s)?,
        })
    }

    /// Get the major version number.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendVersion;
    ///
    /// let version = BackendVersion::new(1, 2, 3);
    /// assert_eq!(version.major(), 1);
    /// ```
    #[must_use]
    pub fn major(&self) -> u64 {
        self.version.major
    }

    /// Get the minor version number.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendVersion;
    ///
    /// let version = BackendVersion::new(1, 2, 3);
    /// assert_eq!(version.minor(), 2);
    /// ```
    #[must_use]
    pub fn minor(&self) -> u64 {
        self.version.minor
    }

    /// Get the patch version number.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendVersion;
    ///
    /// let version = BackendVersion::new(1, 2, 3);
    /// assert_eq!(version.patch(), 3);
    /// ```
    #[must_use]
    pub fn patch(&self) -> u64 {
        self.version.patch
    }

    /// Check if this version is compatible with another version.
    ///
    /// Two versions are compatible if they have the same major version number.
    /// This follows semantic versioning rules where major version changes
    /// indicate breaking changes.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendVersion;
    ///
    /// let v1 = BackendVersion::new(1, 2, 3);
    /// let v2 = BackendVersion::new(1, 3, 0);
    /// let v3 = BackendVersion::new(2, 0, 0);
    ///
    /// assert!(v1.is_compatible_with(&v2));
    /// assert!(!v1.is_compatible_with(&v3));
    /// ```
    #[must_use]
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        self.version.major == other.version.major
    }

    /// Check if this version satisfies a version requirement.
    ///
    /// # Examples
    ///
    /// ```
    /// use canlink_hal::BackendVersion;
    ///
    /// let version = BackendVersion::new(1, 2, 3);
    /// assert!(version.satisfies("^1.0.0").unwrap());
    /// assert!(!version.satisfies("^2.0.0").unwrap());
    /// ```
    ///
    /// # Errors
    ///
    /// Returns an error if the requirement string cannot be parsed.
    pub fn satisfies(&self, req: &str) -> Result<bool, semver::Error> {
        let req = VersionReq::parse(req)?;
        Ok(req.matches(&self.version))
    }

    /// Get the underlying semver Version.
    #[must_use]
    pub const fn as_semver(&self) -> &Version {
        &self.version
    }
}

impl fmt::Display for BackendVersion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.version)
    }
}

impl From<Version> for BackendVersion {
    fn from(version: Version) -> Self {
        Self { version }
    }
}

impl From<BackendVersion> for Version {
    fn from(backend_version: BackendVersion) -> Self {
        backend_version.version
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let version = BackendVersion::new(1, 2, 3);
        assert_eq!(version.major(), 1);
        assert_eq!(version.minor(), 2);
        assert_eq!(version.patch(), 3);
    }

    #[test]
    fn test_version_parse() {
        let version = BackendVersion::parse("1.2.3").unwrap();
        assert_eq!(version.major(), 1);
        assert_eq!(version.minor(), 2);
        assert_eq!(version.patch(), 3);
    }

    #[test]
    fn test_version_display() {
        let version = BackendVersion::new(1, 2, 3);
        assert_eq!(version.to_string(), "1.2.3");
    }

    #[test]
    fn test_version_compatibility() {
        let v1 = BackendVersion::new(1, 2, 3);
        let v2 = BackendVersion::new(1, 3, 0);
        let v3 = BackendVersion::new(2, 0, 0);

        assert!(v1.is_compatible_with(&v2));
        assert!(v2.is_compatible_with(&v1));
        assert!(!v1.is_compatible_with(&v3));
        assert!(!v3.is_compatible_with(&v1));
    }

    #[test]
    fn test_version_satisfies() {
        let version = BackendVersion::new(1, 2, 3);

        assert!(version.satisfies("^1.0.0").unwrap());
        assert!(version.satisfies("^1.2.0").unwrap());
        assert!(version.satisfies(">=1.0.0").unwrap());
        assert!(!version.satisfies("^2.0.0").unwrap());
        assert!(!version.satisfies("<1.0.0").unwrap());
    }

    #[test]
    fn test_invalid_version_parse() {
        assert!(BackendVersion::parse("invalid").is_err());
        assert!(BackendVersion::parse("1.2").is_err());
    }
}
