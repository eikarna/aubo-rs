//! Filter list management for aubo-rs

use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::SystemTime;

use log::{debug, error, info, warn};
#[cfg(feature = "network")]
use reqwest;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::config::{FilterListConfig, FilterListType};
use crate::error::{FilterError, Result};

/// Filter list metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterListMetadata {
    pub name: String,
    pub url: Url,
    pub list_type: FilterListType,
    pub last_updated: Option<SystemTime>,
    pub rule_count: usize,
    pub enabled: bool,
}

/// Parsed filter rule
#[derive(Debug, Clone)]
pub struct ParsedRule {
    pub pattern: String,
    pub rule_type: RuleType,
    pub options: Vec<String>,
}

/// Rule types
#[derive(Debug, Clone)]
pub enum RuleType {
    Block,
    Allow,
    Comment,
    Invalid,
}

/// Filter list manager
pub struct FilterManager {
    lists: HashMap<String, FilterListMetadata>,
    rules_cache: HashMap<String, Vec<ParsedRule>>,
}

impl FilterManager {
    /// Create a new filter manager
    pub fn new() -> Self {
        Self {
            lists: HashMap::new(),
            rules_cache: HashMap::new(),
        }
    }

    /// Add a filter list
    pub fn add_filter_list(&mut self, config: FilterListConfig) -> Result<()> {
        let metadata = FilterListMetadata {
            name: config.name.clone(),
            url: config.url,
            list_type: config.list_type,
            last_updated: None,
            rule_count: 0,
            enabled: config.enabled,
        };

        self.lists.insert(config.name, metadata);
        Ok(())
    }

    /// Download and parse a filter list
    pub async fn update_filter_list(&mut self, name: &str) -> Result<()> {
        let metadata = self.lists.get(name).ok_or_else(|| FilterError::ListNotFound {
            name: name.to_string(),
        })?;

        if !metadata.enabled {
            debug!("Skipping disabled filter list: {}", name);
            return Ok(());
        }

        info!("Updating filter list: {}", name);

        // Download the filter list
        let content = self.download_filter_list(&metadata.url).await?;
        
        // Parse the rules
        let rules = self.parse_filter_content(&content, &metadata.list_type)?;
        
        // Cache the rules
        self.rules_cache.insert(name.to_string(), rules.clone());
        
        // Update metadata
        if let Some(metadata) = self.lists.get_mut(name) {
            metadata.last_updated = Some(SystemTime::now());
            metadata.rule_count = rules.len();
        }

        info!("Updated filter list '{}' with {} rules", name, rules.len());
        Ok(())
    }

    /// Download filter list content
    #[cfg(feature = "network")]
    async fn download_filter_list(&self, url: &Url) -> Result<String> {
        let response = reqwest::get(url.as_str())
            .await
            .map_err(|e| FilterError::DownloadFailed {
                name: "unknown".to_string(),
                url: url.to_string(),
                reason: e.to_string(),
            })?;

        let content = response
            .text()
            .await
            .map_err(|e| FilterError::DownloadFailed {
                name: "unknown".to_string(),
                url: url.to_string(),
                reason: e.to_string(),
            })?;

        Ok(content)
    }

    /// Download filter list content (stub when network feature is disabled)
    #[cfg(not(feature = "network"))]
    async fn download_filter_list(&self, url: &Url) -> Result<String> {
        Err(FilterError::DownloadFailed {
            name: "unknown".to_string(),
            url: url.to_string(),
            reason: "Network feature disabled".to_string(),
        })
    }

    /// Load filter list from local file
    pub fn load_filter_list_from_file(&mut self, name: &str, file_path: &Path) -> Result<()> {
        let content = std::fs::read_to_string(file_path)
            .map_err(|e| FilterError::UpdateFailed {
                reason: format!("Failed to read file {}: {}", file_path.display(), e),
            })?;

        let metadata = self.lists.get(name).ok_or_else(|| FilterError::ListNotFound {
            name: name.to_string(),
        })?;

        // Parse the rules
        let rules = self.parse_filter_content(&content, &metadata.list_type)?;
        
        // Cache the rules
        self.rules_cache.insert(name.to_string(), rules.clone());
        
        // Update metadata
        if let Some(metadata) = self.lists.get_mut(name) {
            metadata.last_updated = Some(SystemTime::now());
            metadata.rule_count = rules.len();
        }

        info!("Loaded filter list '{}' from file with {} rules", name, rules.len());
        Ok(())
    }

    /// Parse filter list content based on type
    fn parse_filter_content(&self, content: &str, list_type: &FilterListType) -> Result<Vec<ParsedRule>> {
        match list_type {
            FilterListType::EasyList => self.parse_easylist_format(content),
            FilterListType::AdGuard => self.parse_adguard_format(content),
            FilterListType::Hosts => self.parse_hosts_format(content),
            FilterListType::UBlockOrigin => self.parse_ublock_format(content),
            FilterListType::Custom => self.parse_custom_format(content),
        }
    }

    /// Parse EasyList format
    fn parse_easylist_format(&self, content: &str) -> Result<Vec<ParsedRule>> {
        let mut rules = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('!') {
                continue;
            }

            if line.starts_with("@@") {
                // Allow rule
                rules.push(ParsedRule {
                    pattern: line[2..].to_string(),
                    rule_type: RuleType::Allow,
                    options: Vec::new(),
                });
            } else {
                // Block rule
                rules.push(ParsedRule {
                    pattern: line.to_string(),
                    rule_type: RuleType::Block,
                    options: Vec::new(),
                });
            }
        }

        Ok(rules)
    }

    /// Parse AdGuard format
    fn parse_adguard_format(&self, content: &str) -> Result<Vec<ParsedRule>> {
        // AdGuard format is similar to EasyList for basic rules
        self.parse_easylist_format(content)
    }

    /// Parse hosts file format
    fn parse_hosts_format(&self, content: &str) -> Result<Vec<ParsedRule>> {
        let mut rules = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                let domain = parts[1];
                rules.push(ParsedRule {
                    pattern: domain.to_string(),
                    rule_type: RuleType::Block,
                    options: Vec::new(),
                });
            }
        }

        Ok(rules)
    }

    /// Parse uBlock Origin format
    fn parse_ublock_format(&self, content: &str) -> Result<Vec<ParsedRule>> {
        // uBlock Origin uses EasyList-compatible format for basic rules
        self.parse_easylist_format(content)
    }

    /// Parse custom format
    fn parse_custom_format(&self, content: &str) -> Result<Vec<ParsedRule>> {
        // Simple line-by-line domain blocking
        let mut rules = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            rules.push(ParsedRule {
                pattern: line.to_string(),
                rule_type: RuleType::Block,
                options: Vec::new(),
            });
        }

        Ok(rules)
    }

    /// Get all rules from all enabled filter lists
    pub fn get_all_rules(&self) -> Vec<ParsedRule> {
        self.rules_cache
            .values()
            .flatten()
            .cloned()
            .collect()
    }

    /// Get metadata for all filter lists
    pub fn get_metadata(&self) -> &HashMap<String, FilterListMetadata> {
        &self.lists
    }
}