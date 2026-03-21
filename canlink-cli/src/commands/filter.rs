//! Filter management commands (T055)
//!
//! Provides commands for managing CAN message filters:
//! - `canlink filter add <type> <params>` - Add a filter
//! - `canlink filter list` - List current filters
//! - `canlink filter clear` - Clear all filters

use crate::error::{CliError, CliResult};
use crate::output::OutputFormatter;
use canlink_hal::filter::{FilterChain, IdFilter, RangeFilter};
use serde::Serialize;
use std::sync::{Arc, RwLock};

/// Global filter chain for CLI session
static FILTER_CHAIN: std::sync::OnceLock<Arc<RwLock<FilterChain>>> = std::sync::OnceLock::new();

/// Get or initialize the global filter chain
fn get_filter_chain() -> &'static Arc<RwLock<FilterChain>> {
    FILTER_CHAIN.get_or_init(|| Arc::new(RwLock::new(FilterChain::default())))
}

/// Filter type for CLI
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FilterType {
    /// Single ID filter
    Id,
    /// ID with mask filter
    Mask,
    /// ID range filter
    Range,
}

impl std::str::FromStr for FilterType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "id" => Ok(FilterType::Id),
            "mask" => Ok(FilterType::Mask),
            "range" => Ok(FilterType::Range),
            _ => Err(format!(
                "Unknown filter type '{}'. Valid types: id, mask, range",
                s
            )),
        }
    }
}

/// Output for filter list
#[derive(Serialize)]
pub struct FilterListOutput {
    /// Current filters in evaluation order.
    pub filters: Vec<FilterInfo>,
    /// Total configured filter count.
    pub total_count: usize,
    /// Number of hardware filters.
    pub hardware_count: usize,
    /// Number of software filters.
    pub software_count: usize,
    /// Maximum hardware filters supported by backend.
    pub max_hardware: usize,
}

/// Information about a single filter
#[derive(Serialize)]
pub struct FilterInfo {
    /// Index in the filter chain.
    pub index: usize,
    /// Filter kind (`id`, `mask`, `range`).
    pub filter_type: String,
    /// Human-readable filter expression.
    pub description: String,
    /// Whether this filter can be offloaded to hardware.
    pub is_hardware: bool,
    /// Filter priority in chain evaluation.
    pub priority: u32,
}

/// Output for filter add operation
#[derive(Serialize)]
pub struct FilterAddOutput {
    /// Operation status.
    pub status: String,
    /// Added filter kind.
    pub filter_type: String,
    /// Human-readable filter expression.
    pub description: String,
    /// Index assigned to the new filter.
    pub index: usize,
    /// Total configured filter count after add.
    pub total_count: usize,
}

/// Output for filter clear operation
#[derive(Serialize)]
pub struct FilterClearOutput {
    /// Operation status.
    pub status: String,
    /// Number of removed filters.
    pub cleared_count: usize,
}

/// Parse a hex or decimal value
fn parse_id(s: &str) -> Result<u32, CliError> {
    let s = s.trim();
    if s.starts_with("0x") || s.starts_with("0X") {
        u32::from_str_radix(&s[2..], 16)
            .map_err(|e| CliError::ParseError(format!("Invalid hex ID '{}': {}", s, e)))
    } else {
        s.parse::<u32>()
            .map_err(|e| CliError::ParseError(format!("Invalid ID '{}': {}", s, e)))
    }
}

/// Execute the filter add command
///
/// # Arguments
///
/// * `filter_type` - Type of filter (id, mask, range)
/// * `params` - Filter parameters (depends on type)
/// * `extended` - Whether to use extended frame IDs
/// * `formatter` - Output formatter
pub fn execute_add(
    filter_type: FilterType,
    params: &[String],
    extended: bool,
    formatter: &OutputFormatter,
) -> CliResult<()> {
    let chain = get_filter_chain();
    let mut chain = chain
        .write()
        .map_err(|e| CliError::ConfigError(format!("Failed to lock filter chain: {}", e)))?;

    let (filter_desc, filter_type_str): (String, &str) = match filter_type {
        FilterType::Id => {
            if params.is_empty() {
                return Err(CliError::InvalidArgument(
                    "ID filter requires an ID parameter".to_string(),
                ));
            }
            let id = parse_id(&params[0])?;
            let filter = if extended {
                IdFilter::try_new_extended(id)
                    .map_err(|e| CliError::InvalidArgument(format!("Invalid extended ID: {}", e)))?
            } else {
                IdFilter::try_new(id)
                    .map_err(|e| CliError::InvalidArgument(format!("Invalid ID: {}", e)))?
            };
            let desc = if extended {
                format!("ID=0x{:08X} (extended)", id)
            } else {
                format!("ID=0x{:03X}", id)
            };
            chain.add_filter(Box::new(filter));
            (desc, "id")
        }
        FilterType::Mask => {
            if params.len() < 2 {
                return Err(CliError::InvalidArgument(
                    "Mask filter requires ID and MASK parameters".to_string(),
                ));
            }
            let id = parse_id(&params[0])?;
            let mask = parse_id(&params[1])?;
            let filter = if extended {
                IdFilter::with_mask_extended(id, mask)
            } else {
                IdFilter::with_mask(id, mask)
            };
            let desc = if extended {
                format!("ID=0x{:08X} MASK=0x{:08X} (extended)", id, mask)
            } else {
                format!("ID=0x{:03X} MASK=0x{:03X}", id, mask)
            };
            chain.add_filter(Box::new(filter));
            (desc, "mask")
        }
        FilterType::Range => {
            if params.len() < 2 {
                return Err(CliError::InvalidArgument(
                    "Range filter requires START and END parameters".to_string(),
                ));
            }
            let start = parse_id(&params[0])?;
            let end = parse_id(&params[1])?;
            if start > end {
                return Err(CliError::InvalidArgument(format!(
                    "Start ID (0x{:X}) must be <= End ID (0x{:X})",
                    start, end
                )));
            }
            let filter = if extended {
                RangeFilter::new_extended(start, end)
            } else {
                RangeFilter::new(start, end)
            };
            let desc = if extended {
                format!("RANGE=0x{:08X}-0x{:08X} (extended)", start, end)
            } else {
                format!("RANGE=0x{:03X}-0x{:03X}", start, end)
            };
            chain.add_filter(Box::new(filter));
            (desc, "range")
        }
    };

    let index = chain.len() - 1;
    let total = chain.len();

    if formatter.is_json() {
        let output = FilterAddOutput {
            status: "success".to_string(),
            filter_type: filter_type_str.to_string(),
            description: filter_desc.clone(),
            index,
            total_count: total,
        };
        formatter.print(&output)?;
    } else {
        formatter.print_success(&format!(
            "Added {} filter [{}]: {}",
            filter_type_str, index, filter_desc
        ))?;
        println!("  Total filters: {}", total);
    }

    Ok(())
}

/// Execute the filter list command
pub fn execute_list(formatter: &OutputFormatter) -> CliResult<()> {
    let chain = get_filter_chain();
    let chain = chain
        .read()
        .map_err(|e| CliError::ConfigError(format!("Failed to lock filter chain: {}", e)))?;

    let filters: Vec<FilterInfo> = chain
        .iter()
        .enumerate()
        .map(|(i, f)| {
            // Determine filter type and description from the filter
            let (filter_type, description) = describe_filter(f);
            FilterInfo {
                index: i,
                filter_type,
                description,
                is_hardware: f.is_hardware(),
                priority: f.priority(),
            }
        })
        .collect();

    if formatter.is_json() {
        let output = FilterListOutput {
            total_count: chain.len(),
            hardware_count: chain.hardware_filter_count(),
            software_count: chain.software_filter_count(),
            max_hardware: chain.max_hardware_filters(),
            filters,
        };
        formatter.print(&output)?;
    } else if chain.is_empty() {
        formatter.print_message("No filters configured (all messages pass)")?;
    } else {
        println!("Configured filters ({} total):", chain.len());
        println!(
            "  Hardware: {}/{}, Software: {}",
            chain.hardware_filter_count(),
            chain.max_hardware_filters(),
            chain.software_filter_count()
        );
        println!();
        for info in &filters {
            let hw_sw = if info.is_hardware { "HW" } else { "SW" };
            println!(
                "  [{}] {} ({}) - {} (priority: {})",
                info.index, info.filter_type, hw_sw, info.description, info.priority
            );
        }
    }

    Ok(())
}

/// Execute the filter remove command
pub fn execute_remove(index: usize, formatter: &OutputFormatter) -> CliResult<()> {
    let chain = get_filter_chain();
    let mut chain = chain
        .write()
        .map_err(|e| CliError::ConfigError(format!("Failed to lock filter chain: {}", e)))?;

    chain.remove_filter(index).map_err(|e| {
        CliError::InvalidArgument(format!("Failed to remove filter at index {}: {}", index, e))
    })?;

    if formatter.is_json() {
        let output = serde_json::json!({
            "status": "success",
            "removed_index": index,
            "remaining_count": chain.len()
        });
        formatter.print(&output)?;
    } else {
        formatter.print_success(&format!("Removed filter at index {}", index))?;
        println!("  Remaining filters: {}", chain.len());
    }

    Ok(())
}

/// Execute the filter clear command
pub fn execute_clear(formatter: &OutputFormatter) -> CliResult<()> {
    let chain = get_filter_chain();
    let mut chain = chain
        .write()
        .map_err(|e| CliError::ConfigError(format!("Failed to lock filter chain: {}", e)))?;

    let count = chain.len();
    chain.clear();

    if formatter.is_json() {
        let output = FilterClearOutput {
            status: "success".to_string(),
            cleared_count: count,
        };
        formatter.print(&output)?;
    } else {
        formatter.print_success(&format!("Cleared {} filter(s)", count))?;
    }

    Ok(())
}

/// Describe a filter (type and description)
fn describe_filter(filter: &dyn canlink_hal::filter::MessageFilter) -> (String, String) {
    // Since we can't downcast trait objects easily, we use priority as a hint
    // Priority 100 = exact ID match, Priority 50 = mask match, Priority 0 = range
    let priority = filter.priority();
    if priority >= 100 {
        ("id".to_string(), "ID filter (exact match)".to_string())
    } else if priority >= 50 {
        ("mask".to_string(), "ID filter (mask match)".to_string())
    } else {
        ("range".to_string(), "Range filter".to_string())
    }
}

/// Returns the global filter chain used by CLI commands.
#[allow(dead_code)]
pub fn get_global_filter_chain() -> &'static Arc<RwLock<FilterChain>> {
    get_filter_chain()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_filter_type_from_str() {
        assert_eq!("id".parse::<FilterType>().unwrap(), FilterType::Id);
        assert_eq!("ID".parse::<FilterType>().unwrap(), FilterType::Id);
        assert_eq!("mask".parse::<FilterType>().unwrap(), FilterType::Mask);
        assert_eq!("range".parse::<FilterType>().unwrap(), FilterType::Range);
        assert!("invalid".parse::<FilterType>().is_err());
    }

    #[test]
    fn test_parse_id_hex() {
        assert_eq!(parse_id("0x123").unwrap(), 0x123);
        assert_eq!(parse_id("0X456").unwrap(), 0x456);
    }

    #[test]
    fn test_parse_id_decimal() {
        assert_eq!(parse_id("123").unwrap(), 123);
        assert_eq!(parse_id("456").unwrap(), 456);
    }

    #[test]
    fn test_parse_id_invalid() {
        assert!(parse_id("xyz").is_err());
        assert!(parse_id("0xGGG").is_err());
    }
}
