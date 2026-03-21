//! Filter configuration (FR-006)
//!
//! Provides configuration structures for loading filters from TOML.

use serde::Deserialize;

use crate::error::FilterError;

use super::{FilterChain, IdFilter, RangeFilter};

/// Filter configuration from TOML
///
/// # Example TOML
///
/// ```toml
/// [filters]
/// [[filters.id_filters]]
/// id = 0x123
/// mask = 0x7FF
/// extended = false
///
/// [[filters.range_filters]]
/// start_id = 0x200
/// end_id = 0x2FF
/// ```
#[derive(Debug, Clone, Default, Deserialize)]
pub struct FilterConfig {
    /// ID filters
    #[serde(default)]
    pub id_filters: Vec<IdFilterConfig>,

    /// Range filters
    #[serde(default)]
    pub range_filters: Vec<RangeFilterConfig>,

    /// Maximum hardware filters (default: 4)
    #[serde(default = "default_max_hardware")]
    pub max_hardware_filters: usize,
}

fn default_max_hardware() -> usize {
    4
}

/// ID filter configuration
#[derive(Debug, Clone, Deserialize)]
pub struct IdFilterConfig {
    /// Target ID
    pub id: u32,

    /// Mask (default: 0x7FF for standard, 0x1FFFFFFF for extended)
    #[serde(default)]
    pub mask: Option<u32>,

    /// Extended frame (default: false)
    #[serde(default)]
    pub extended: bool,
}

/// Range filter configuration
#[derive(Debug, Clone, Deserialize)]
pub struct RangeFilterConfig {
    /// Start ID (inclusive)
    pub start_id: u32,

    /// End ID (inclusive)
    pub end_id: u32,

    /// Extended frame (default: false)
    #[serde(default)]
    pub extended: bool,
}

impl FilterConfig {
    /// Load configuration from TOML string
    ///
    /// # Errors
    ///
    /// Returns an error if the TOML string is invalid.
    pub fn from_toml(toml_str: &str) -> Result<Self, toml::de::Error> {
        toml::from_str(toml_str)
    }

    /// Build a `FilterChain` from this configuration
    ///
    /// # Errors
    ///
    /// Returns `FilterError` if any filter configuration is invalid.
    pub fn into_chain(self) -> Result<FilterChain, FilterError> {
        let mut chain = FilterChain::new(self.max_hardware_filters);

        // Add ID filters
        for config in self.id_filters {
            let filter = if config.extended {
                if let Some(mask) = config.mask {
                    IdFilter::with_mask_extended(config.id, mask)
                } else {
                    IdFilter::try_new_extended(config.id)?
                }
            } else if let Some(mask) = config.mask {
                IdFilter::with_mask(config.id, mask)
            } else {
                IdFilter::try_new(config.id)?
            };
            chain.add_filter(Box::new(filter));
        }

        // Add range filters
        for config in self.range_filters {
            let filter = if config.extended {
                RangeFilter::try_new_extended(config.start_id, config.end_id)?
            } else {
                RangeFilter::try_new(config.start_id, config.end_id)?
            };
            chain.add_filter(Box::new(filter));
        }

        Ok(chain)
    }
}

impl FilterChain {
    /// Create a `FilterChain` from configuration
    ///
    /// # Errors
    ///
    /// Returns `FilterError` if any filter configuration is invalid.
    pub fn from_config(config: &FilterConfig) -> Result<Self, FilterError> {
        config.clone().into_chain()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::message::CanMessage;

    #[test]
    fn test_parse_id_filter() {
        let toml = r"
            [[id_filters]]
            id = 0x123
        ";

        let config: FilterConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.id_filters.len(), 1);
        assert_eq!(config.id_filters[0].id, 0x123);
    }

    #[test]
    fn test_parse_id_filter_with_mask() {
        let toml = r"
            [[id_filters]]
            id = 0x120
            mask = 0x7F0
        ";

        let config: FilterConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.id_filters[0].mask, Some(0x7F0));
    }

    #[test]
    fn test_parse_range_filter() {
        let toml = r"
            [[range_filters]]
            start_id = 0x100
            end_id = 0x1FF
        ";

        let config: FilterConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.range_filters.len(), 1);
        assert_eq!(config.range_filters[0].start_id, 0x100);
        assert_eq!(config.range_filters[0].end_id, 0x1FF);
    }

    #[test]
    fn test_into_chain() {
        let toml = r"
            max_hardware_filters = 2

            [[id_filters]]
            id = 0x123

            [[range_filters]]
            start_id = 0x200
            end_id = 0x2FF
        ";

        let config: FilterConfig = toml::from_str(toml).unwrap();
        let chain = config.into_chain().unwrap();

        assert_eq!(chain.len(), 2);

        let msg_123 = CanMessage::new_standard(0x123, &[0u8; 8]).unwrap();
        let msg_250 = CanMessage::new_standard(0x250, &[0u8; 8]).unwrap();
        let msg_300 = CanMessage::new_standard(0x300, &[0u8; 8]).unwrap();

        assert!(chain.matches(&msg_123));
        assert!(chain.matches(&msg_250));
        assert!(!chain.matches(&msg_300));
    }

    #[test]
    fn test_extended_filters() {
        let toml = r"
            [[id_filters]]
            id = 0x12345678
            extended = true

            [[range_filters]]
            start_id = 0x10000
            end_id = 0x1FFFF
            extended = true
        ";

        let config: FilterConfig = toml::from_str(toml).unwrap();
        let chain = config.into_chain().unwrap();

        let msg_ext = CanMessage::new_extended(0x1234_5678, &[0u8; 8]).unwrap();
        let msg_range = CanMessage::new_extended(0x15000, &[0u8; 8]).unwrap();

        assert!(chain.matches(&msg_ext));
        assert!(chain.matches(&msg_range));
    }
}
